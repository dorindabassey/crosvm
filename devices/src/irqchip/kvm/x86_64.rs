// Copyright 2020 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::sync::Arc;

use sync::Mutex;

#[cfg(not(test))]
use base::Clock;
#[cfg(test)]
use base::FakeClock as Clock;
use hypervisor::kvm::{KvmVcpu, KvmVm};
use hypervisor::{
    IoapicState, IrqRoute, IrqSource, IrqSourceChip, LapicState, MPState, PicSelect, PicState,
    PitState, Vm, NUM_IOAPIC_PINS,
};
use kvm_sys::*;
use resources::SystemAllocator;

use base::{error, Error, EventFd, Result};
use vm_control::VmIrqRequestSocket;

use crate::irqchip::{Ioapic, Pic, IOAPIC_BASE_ADDRESS, IOAPIC_MEM_LENGTH_BYTES};
use crate::{Bus, IrqChip, IrqChipX86_64, Pit, PitError};

/// PIT channel 0 timer is connected to IRQ 0
const PIT_CHANNEL0_IRQ: u32 = 0;

/// Default x86 routing table.  Pins 0-7 go to primary pic and ioapic, pins 8-15 go to secondary
/// pic and ioapic, and pins 16-23 go only to the ioapic.
fn kvm_default_irq_routing_table() -> Vec<IrqRoute> {
    let mut routes: Vec<IrqRoute> = Vec::new();

    for i in 0..8 {
        routes.push(IrqRoute::pic_irq_route(IrqSourceChip::PicPrimary, i));
        routes.push(IrqRoute::ioapic_irq_route(i));
    }
    for i in 8..16 {
        routes.push(IrqRoute::pic_irq_route(IrqSourceChip::PicSecondary, i));
        routes.push(IrqRoute::ioapic_irq_route(i));
    }
    for i in 16..NUM_IOAPIC_PINS as u32 {
        routes.push(IrqRoute::ioapic_irq_route(i));
    }

    routes
}

/// IrqChip implementation where the entire IrqChip is emulated by KVM.
///
/// This implementation will use the KVM API to create and configure the in-kernel irqchip.
pub struct KvmKernelIrqChip {
    pub(super) vm: KvmVm,
    pub(super) vcpus: Arc<Mutex<Vec<Option<KvmVcpu>>>>,
    pub(super) routes: Arc<Mutex<Vec<IrqRoute>>>,
}

impl KvmKernelIrqChip {
    /// Construct a new KvmKernelIrqchip.
    pub fn new(vm: KvmVm, num_vcpus: usize) -> Result<KvmKernelIrqChip> {
        vm.create_irq_chip()?;
        vm.create_pit()?;

        Ok(KvmKernelIrqChip {
            vm,
            vcpus: Arc::new(Mutex::new((0..num_vcpus).map(|_| None).collect())),
            routes: Arc::new(Mutex::new(kvm_default_irq_routing_table())),
        })
    }
    /// Attempt to create a shallow clone of this x86_64 KvmKernelIrqChip instance.
    pub(super) fn arch_try_clone(&self) -> Result<Self> {
        Ok(KvmKernelIrqChip {
            vm: self.vm.try_clone()?,
            vcpus: self.vcpus.clone(),
            routes: self.routes.clone(),
        })
    }
}

impl IrqChipX86_64<KvmVcpu> for KvmKernelIrqChip {
    /// Get the current state of the PIC
    fn get_pic_state(&self, select: PicSelect) -> Result<PicState> {
        Ok(PicState::from(&self.vm.get_pic_state(select)?))
    }

    /// Set the current state of the PIC
    fn set_pic_state(&mut self, select: PicSelect, state: &PicState) -> Result<()> {
        self.vm.set_pic_state(select, &kvm_pic_state::from(state))
    }

    /// Get the current state of the IOAPIC
    fn get_ioapic_state(&self) -> Result<IoapicState> {
        Ok(IoapicState::from(&self.vm.get_ioapic_state()?))
    }

    /// Set the current state of the IOAPIC
    fn set_ioapic_state(&mut self, state: &IoapicState) -> Result<()> {
        self.vm.set_ioapic_state(&kvm_ioapic_state::from(state))
    }

    /// Get the current state of the specified VCPU's local APIC
    fn get_lapic_state(&self, vcpu_id: usize) -> Result<LapicState> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => Ok(LapicState::from(&vcpu.get_lapic()?)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Set the current state of the specified VCPU's local APIC
    fn set_lapic_state(&mut self, vcpu_id: usize, state: &LapicState) -> Result<()> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => vcpu.set_lapic(&kvm_lapic_state::from(state)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Retrieves the state of the PIT. Gets the pit state via the KVM API.
    fn get_pit(&self) -> Result<PitState> {
        Ok(PitState::from(&self.vm.get_pit_state()?))
    }

    /// Sets the state of the PIT. Sets the pit state via the KVM API.
    fn set_pit(&mut self, state: &PitState) -> Result<()> {
        self.vm.set_pit_state(&kvm_pit_state2::from(state))
    }
}

/// The KvmSplitIrqsChip supports KVM's SPLIT_IRQCHIP feature, where the PIC and IOAPIC
/// are emulated in userspace, while the local APICs are emulated in the kernel.
/// The SPLIT_IRQCHIP feature only supports x86/x86_64 so we only define this IrqChip in crosvm
/// for x86/x86_64.
pub struct KvmSplitIrqChip {
    vm: KvmVm,
    vcpus: Arc<Mutex<Vec<Option<KvmVcpu>>>>,
    routes: Arc<Mutex<Vec<IrqRoute>>>,
    pit: Arc<Mutex<Pit>>,
    pic: Arc<Mutex<Pic>>,
    ioapic: Arc<Mutex<Ioapic>>,
    /// Vec of ioapic irq events that have been delayed because the ioapic was locked when
    /// service_irq was called on the irqchip. This prevents deadlocks when a Vcpu thread has
    /// locked the ioapic and the ioapic sends a AddMsiRoute signal to the main thread (which
    /// itself may be busy trying to call service_irq).
    delayed_ioapic_irq_events: Arc<Mutex<Vec<usize>>>,
    /// Vec of EventFds that the ioapic will use to trigger interrupts in KVM. This is not
    /// wrapped in an Arc<Mutex<>> because the EventFds themselves can be cloned and they will
    /// not change after the IrqChip is created.
    irqfds: Vec<EventFd>,
    /// Array of EventFds that devices will use to assert ioapic pins.
    irq_events: Arc<Mutex<[Option<EventFd>; NUM_IOAPIC_PINS]>>,
    /// Array of EventFds that should be asserted when the ioapic receives an EOI.
    resample_events: Arc<Mutex<[Option<EventFd>; NUM_IOAPIC_PINS]>>,
}

fn kvm_dummy_msi_routes() -> Vec<IrqRoute> {
    let mut routes: Vec<IrqRoute> = Vec::new();
    for i in 0..NUM_IOAPIC_PINS {
        routes.push(
            // Add dummy MSI routes to replace the default IRQChip routes.
            IrqRoute {
                gsi: i as u32,
                source: IrqSource::Msi {
                    address: 0,
                    data: 0,
                },
            },
        );
    }
    routes
}
impl KvmSplitIrqChip {
    /// Construct a new KvmSplitIrqChip.
    pub fn new(vm: KvmVm, num_vcpus: usize, irq_socket: VmIrqRequestSocket) -> Result<Self> {
        vm.enable_split_irqchip()?;

        let pit_evt = EventFd::new()?;
        let pit = Arc::new(Mutex::new(
            Pit::new(pit_evt.try_clone()?, Arc::new(Mutex::new(Clock::new()))).map_err(
                |e| match e {
                    PitError::CloneEventFd(err) => err,
                    PitError::CreateEventFd(err) => err,
                    PitError::CreatePollContext(err) => err,
                    PitError::PollError(err) => err,
                    PitError::TimerFdCreateError(err) => err,
                    PitError::SpawnThread(_) => Error::new(libc::EIO),
                },
            )?,
        ));
        let mut irqfds: Vec<EventFd> = Vec::with_capacity(NUM_IOAPIC_PINS);
        let mut irqfds_for_ioapic: Vec<EventFd> = Vec::with_capacity(NUM_IOAPIC_PINS);

        for i in 0..NUM_IOAPIC_PINS {
            let evt = EventFd::new()?;
            vm.register_irqfd(i as u32, &evt, None)?;
            irqfds_for_ioapic.push(evt.try_clone()?);
            irqfds.push(evt);
        }

        let mut chip = KvmSplitIrqChip {
            vm,
            vcpus: Arc::new(Mutex::new((0..num_vcpus).map(|_| None).collect())),
            routes: Arc::new(Mutex::new(Vec::new())),
            pit,
            pic: Arc::new(Mutex::new(Pic::new())),
            ioapic: Arc::new(Mutex::new(Ioapic::new(irqfds_for_ioapic, irq_socket)?)),
            delayed_ioapic_irq_events: Arc::new(Mutex::new(Vec::new())),
            irqfds,
            irq_events: Arc::new(Mutex::new(Default::default())),
            resample_events: Arc::new(Mutex::new(Default::default())),
        };

        // Setup standard x86 irq routes
        let mut routes = kvm_default_irq_routing_table();
        // Add dummy MSI routes for the first 24 GSIs
        routes.append(&mut kvm_dummy_msi_routes());

        // Set the routes so they get sent to KVM
        chip.set_irq_routes(&routes)?;

        chip.register_irq_event(PIT_CHANNEL0_IRQ, &pit_evt, None)?;
        Ok(chip)
    }
}

impl KvmSplitIrqChip {
    /// Convenience function for determining which chips the supplied irq routes to.
    fn routes_to_chips(&self, irq: u32) -> Vec<(IrqSourceChip, u32)> {
        let mut chips = Vec::new();
        for route in self.routes.lock().iter() {
            match route {
                IrqRoute {
                    gsi,
                    source: IrqSource::Irqchip { chip, pin },
                } if *gsi == irq => match chip {
                    IrqSourceChip::PicPrimary
                    | IrqSourceChip::PicSecondary
                    | IrqSourceChip::Ioapic => chips.push((*chip, *pin)),
                    IrqSourceChip::Gic => {
                        error!("gic irq should not be possible on a KvmSplitIrqChip")
                    }
                },
                // Ignore MSIs and other routes
                _ => {}
            }
        }
        chips
    }
}

/// Convenience function for determining whether or not two irq routes conflict.
/// Returns true if they conflict.
fn routes_conflict(route: &IrqRoute, other: &IrqRoute) -> bool {
    // They don't conflict if they have different GSIs.
    if route.gsi != other.gsi {
        return false;
    }

    // If they're both MSI with the same GSI then they conflict.
    if let (IrqSource::Msi { .. }, IrqSource::Msi { .. }) = (route.source, other.source) {
        return true;
    }

    // If the route chips match and they have the same GSI then they conflict.
    if let (
        IrqSource::Irqchip {
            chip: route_chip, ..
        },
        IrqSource::Irqchip {
            chip: other_chip, ..
        },
    ) = (route.source, other.source)
    {
        return route_chip == other_chip;
    }

    // Otherwise they do not conflict.
    false
}

/// This IrqChip only works with Kvm so we only implement it for KvmVcpu.
impl IrqChip<KvmVcpu> for KvmSplitIrqChip {
    /// Add a vcpu to the irq chip.
    fn add_vcpu(&mut self, vcpu_id: usize, vcpu: KvmVcpu) -> Result<()> {
        self.vcpus.lock()[vcpu_id] = Some(vcpu);
        Ok(())
    }

    /// Register an event that can trigger an interrupt for a particular GSI.
    fn register_irq_event(
        &mut self,
        irq: u32,
        irq_event: &EventFd,
        resample_event: Option<&EventFd>,
    ) -> Result<()> {
        if irq < NUM_IOAPIC_PINS as u32 {
            // safe to index here because irq_events is NUM_IOAPIC_PINS long
            self.irq_events.lock()[irq as usize] = Some(irq_event.try_clone()?);
            if let Some(evt) = resample_event {
                self.resample_events.lock()[irq as usize] = Some(evt.try_clone()?);
            }

            Ok(())
        } else {
            self.vm.register_irqfd(irq, irq_event, resample_event)
        }
    }

    /// Unregister an event for a particular GSI.
    fn unregister_irq_event(&mut self, irq: u32, irq_event: &EventFd) -> Result<()> {
        if irq < NUM_IOAPIC_PINS as u32 {
            match &self.irq_events.lock()[irq as usize] {
                // We only do something if irq_event is the same as our existing event
                Some(evt) if evt == irq_event => {
                    // safe to index here because irq_events is NUM_IOAPIC_PINS long
                    self.irq_events.lock()[irq as usize] = None;
                    self.resample_events.lock()[irq as usize] = None;
                    Ok(())
                }
                _ => Ok(()),
            }
        } else {
            self.vm.unregister_irqfd(irq, irq_event)
        }
    }

    /// Route an IRQ line to an interrupt controller, or to a particular MSI vector.
    fn route_irq(&mut self, route: IrqRoute) -> Result<()> {
        let mut routes = self.routes.lock();
        routes.retain(|r| !routes_conflict(r, &route));

        routes.push(route);

        // We only call set_gsi_routing with the msi routes
        let mut msi_routes = routes.clone();
        msi_routes.retain(|r| match r.source {
            IrqSource::Msi { .. } => true,
            _ => false,
        });

        self.vm.set_gsi_routing(&*msi_routes)
    }

    /// Replace all irq routes with the supplied routes
    fn set_irq_routes(&mut self, routes: &[IrqRoute]) -> Result<()> {
        let mut current_routes = self.routes.lock();
        *current_routes = routes.to_vec();

        // We only call set_gsi_routing with the msi routes
        let mut msi_routes = routes.to_vec().clone();
        msi_routes.retain(|r| match r.source {
            IrqSource::Msi { .. } => true,
            _ => false,
        });

        self.vm.set_gsi_routing(&*msi_routes)
    }

    /// Return a vector of all registered irq numbers and their associated events. To be used by
    /// the main thread to wait for irq events to be triggered.
    fn irq_event_tokens(&self) -> Result<Vec<(u32, EventFd)>> {
        let mut tokens: Vec<(u32, EventFd)> = Vec::new();
        for i in 0..NUM_IOAPIC_PINS {
            if let Some(evt) = &self.irq_events.lock()[i] {
                tokens.push((i as u32, evt.try_clone()?));
            }
        }
        Ok(tokens)
    }

    /// Either assert or deassert an IRQ line.  Sends to either an interrupt controller, or does
    /// a send_msi if the irq is associated with an MSI.
    fn service_irq(&mut self, irq: u32, level: bool) -> Result<()> {
        let chips = self.routes_to_chips(irq);
        for (chip, pin) in chips {
            match chip {
                IrqSourceChip::PicPrimary | IrqSourceChip::PicSecondary => {
                    self.pic.lock().service_irq(pin as u8, level);
                }
                IrqSourceChip::Ioapic => {
                    self.ioapic.lock().service_irq(pin as usize, level);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Service an IRQ event by asserting then deasserting an IRQ line. The associated EventFd
    /// that triggered the irq event will be read from. If the irq is associated with a resample
    /// EventFd, then the deassert will only happen after an EOI is broadcast for a vector
    /// associated with the irq line.
    /// For the KvmSplitIrqChip, this function identifies which chips the irq routes to, then
    /// attempts to call service_irq on those chips. If the ioapic is unable to be immediately
    /// locked, we add the irq to the delayed_ioapic_irq_events Vec (though we still read
    /// from the EventFd that triggered the irq event).
    fn service_irq_event(&mut self, irq: u32) -> Result<()> {
        if let Some(evt) = &self.irq_events.lock()[irq as usize] {
            evt.read()?;
        }
        let chips = self.routes_to_chips(irq);

        for (chip, pin) in chips {
            match chip {
                IrqSourceChip::PicPrimary | IrqSourceChip::PicSecondary => {
                    let mut pic = self.pic.lock();
                    if self.resample_events.lock()[pin as usize].is_some() {
                        pic.service_irq(pin as u8, true);
                    } else {
                        pic.service_irq(pin as u8, true);
                        pic.service_irq(pin as u8, false);
                    }
                }
                IrqSourceChip::Ioapic => {
                    if let Ok(mut ioapic) = self.ioapic.try_lock() {
                        if self.resample_events.lock()[pin as usize].is_some() {
                            ioapic.service_irq(pin as usize, true);
                        } else {
                            ioapic.service_irq(pin as usize, true);
                            ioapic.service_irq(pin as usize, false);
                        }
                    } else {
                        self.delayed_ioapic_irq_events.lock().push(pin as usize);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Broadcast an end of interrupt. For KvmSplitIrqChip this sends the EOI to the ioapic
    fn broadcast_eoi(&mut self, vector: u8) -> Result<()> {
        self.ioapic.lock().end_of_interrupt(vector);
        Ok(())
    }

    /// Return true if there is a pending interrupt for the specified vcpu. For KvmSplitIrqChip
    /// this calls interrupt_requested on the pic.
    fn interrupt_requested(&self, vcpu_id: usize) -> bool {
        // Pic interrupts for the split irqchip only go to vcpu 0
        if vcpu_id != 0 {
            return false;
        }
        self.pic.lock().interrupt_requested()
    }

    /// Check if the specified vcpu has any pending interrupts. Returns None for no interrupts,
    /// otherwise Some(u32) should be the injected interrupt vector. For KvmSplitIrqChip
    /// this calls get_external_interrupt on the pic.
    fn get_external_interrupt(&mut self, vcpu_id: usize) -> Result<Option<u32>> {
        // Pic interrupts for the split irqchip only go to vcpu 0
        if vcpu_id != 0 {
            return Ok(None);
        }
        if let Some(vector) = self.pic.lock().get_external_interrupt() {
            Ok(Some(vector as u32))
        } else {
            Ok(None)
        }
    }

    /// Get the current MP state of the specified VCPU.
    fn get_mp_state(&self, vcpu_id: usize) -> Result<MPState> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => Ok(MPState::from(&vcpu.get_mp_state()?)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Set the current MP state of the specified VCPU.
    fn set_mp_state(&mut self, vcpu_id: usize, state: &MPState) -> Result<()> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => vcpu.set_mp_state(&kvm_mp_state::from(state)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Attempt to clone this IrqChip instance.
    fn try_clone(&self) -> Result<Self> {
        let mut new_irqfds: Vec<EventFd> = Vec::with_capacity(NUM_IOAPIC_PINS);
        for i in 0..NUM_IOAPIC_PINS {
            new_irqfds.push(self.irqfds[i].try_clone()?);
        }
        Ok(KvmSplitIrqChip {
            vm: self.vm.try_clone()?,
            vcpus: self.vcpus.clone(),
            routes: self.routes.clone(),
            pit: self.pit.clone(),
            pic: self.pic.clone(),
            ioapic: self.ioapic.clone(),
            delayed_ioapic_irq_events: self.delayed_ioapic_irq_events.clone(),
            irqfds: new_irqfds,
            irq_events: self.irq_events.clone(),
            resample_events: self.resample_events.clone(),
        })
    }

    /// Finalize irqchip setup. Should be called once all devices have registered irq events and
    /// been added to the io_bus and mmio_bus.
    fn finalize_devices(
        &mut self,
        resources: &mut SystemAllocator,
        io_bus: &mut Bus,
        mmio_bus: &mut Bus,
    ) -> Result<()> {
        // Insert pit into io_bus
        io_bus.insert(self.pit.clone(), 0x040, 0x8, true).unwrap();
        io_bus.insert(self.pit.clone(), 0x061, 0x1, true).unwrap();

        // Insert pic into io_bus
        io_bus.insert(self.pic.clone(), 0x20, 0x2, true).unwrap();
        io_bus.insert(self.pic.clone(), 0xa0, 0x2, true).unwrap();
        io_bus.insert(self.pic.clone(), 0x4d0, 0x2, true).unwrap();

        // Insert ioapic into mmio_bus
        mmio_bus
            .insert(
                self.ioapic.clone(),
                IOAPIC_BASE_ADDRESS,
                IOAPIC_MEM_LENGTH_BYTES,
                false,
            )
            .unwrap();

        // At this point, all of our devices have been created and they have registered their
        // irq events, so we can clone our resample events
        let mut ioapic_resample_events: Vec<Option<EventFd>> = Vec::with_capacity(NUM_IOAPIC_PINS);
        let mut pic_resample_events: Vec<Option<EventFd>> = Vec::with_capacity(NUM_IOAPIC_PINS);

        for i in 0..NUM_IOAPIC_PINS {
            match &self.resample_events.lock()[i] {
                Some(e) => {
                    ioapic_resample_events.push(Some(e.try_clone()?));
                    pic_resample_events.push(Some(e.try_clone()?));
                }
                None => {
                    ioapic_resample_events.push(None);
                    pic_resample_events.push(None);
                }
            };
        }

        // Register resample events with the ioapic
        self.ioapic
            .lock()
            .register_resample_events(ioapic_resample_events);
        // Register resample events with the pic
        self.pic
            .lock()
            .register_resample_events(pic_resample_events);

        // Make sure all future irq numbers are >= NUM_IOAPIC_PINS
        let mut irq_num = resources.allocate_irq().unwrap();
        while irq_num < NUM_IOAPIC_PINS as u32 {
            irq_num = resources.allocate_irq().unwrap();
        }

        Ok(())
    }

    /// The KvmSplitIrqChip's ioapic may be locked because a vcpu thread is currently writing to
    /// the ioapic, and the ioapic may be blocking on adding MSI routes, which requires blocking
    /// socket communication back to the main thread.  Thus, we do not want the main thread to
    /// block on a locked ioapic, so any irqs that could not be serviced because the ioapic could
    /// not be immediately locked are added to the delayed_ioapic_irq_events Vec. This function
    /// processes each delayed event in the vec each time it's called. If the ioapic is still
    /// locked, we keep the queued irqs for the next time this function is called.
    fn process_delayed_irq_events(&mut self) -> Result<()> {
        self.delayed_ioapic_irq_events.lock().retain(|&irq| {
            if let Ok(mut ioapic) = self.ioapic.try_lock() {
                if self.resample_events.lock()[irq].is_some() {
                    ioapic.service_irq(irq, true);
                } else {
                    ioapic.service_irq(irq, true);
                    ioapic.service_irq(irq, false);
                }
                false
            } else {
                true
            }
        });

        Ok(())
    }
}

impl IrqChipX86_64<KvmVcpu> for KvmSplitIrqChip {
    /// Get the current state of the PIC
    fn get_pic_state(&self, select: PicSelect) -> Result<PicState> {
        Ok(self.pic.lock().get_pic_state(select))
    }

    /// Set the current state of the PIC
    fn set_pic_state(&mut self, select: PicSelect, state: &PicState) -> Result<()> {
        self.pic.lock().set_pic_state(select, state);
        Ok(())
    }

    /// Get the current state of the IOAPIC
    fn get_ioapic_state(&self) -> Result<IoapicState> {
        Ok(self.ioapic.lock().get_ioapic_state())
    }

    /// Set the current state of the IOAPIC
    fn set_ioapic_state(&mut self, state: &IoapicState) -> Result<()> {
        self.ioapic.lock().set_ioapic_state(state);
        Ok(())
    }

    /// Get the current state of the specified VCPU's local APIC
    fn get_lapic_state(&self, vcpu_id: usize) -> Result<LapicState> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => Ok(LapicState::from(&vcpu.get_lapic()?)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Set the current state of the specified VCPU's local APIC
    fn set_lapic_state(&mut self, vcpu_id: usize, state: &LapicState) -> Result<()> {
        match self.vcpus.lock().get(vcpu_id) {
            Some(Some(vcpu)) => vcpu.set_lapic(&kvm_lapic_state::from(state)),
            _ => Err(Error::new(libc::ENOENT)),
        }
    }

    /// Retrieves the state of the PIT. Gets the pit state via the KVM API.
    fn get_pit(&self) -> Result<PitState> {
        Ok(self.pit.lock().get_pit_state())
    }

    /// Sets the state of the PIT. Sets the pit state via the KVM API.
    fn set_pit(&mut self, state: &PitState) -> Result<()> {
        self.pit.lock().set_pit_state(state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use base::EventReadResult;
    use hypervisor::kvm::Kvm;
    use vm_memory::GuestMemory;

    use hypervisor::{IoapicRedirectionTableEntry, PitRWMode, TriggerMode, Vm, VmX86_64};
    use vm_control::{VmIrqRequest, VmIrqResponse};

    use super::super::super::tests::*;
    use crate::IrqChip;

    /// Helper function for setting up a KvmKernelIrqChip
    fn get_kernel_chip() -> KvmKernelIrqChip {
        let kvm = Kvm::new().expect("failed to instantiate Kvm");
        let mem = GuestMemory::new(&[]).unwrap();
        let vm = KvmVm::new(&kvm, mem).expect("failed tso instantiate vm");

        let mut chip = KvmKernelIrqChip::new(vm.try_clone().expect("failed to clone vm"), 1)
            .expect("failed to instantiate KvmKernelIrqChip");

        let vcpu = vm.create_vcpu(0).expect("failed to instantiate vcpu");
        chip.add_vcpu(0, vcpu).expect("failed to add vcpu");

        chip
    }

    /// Helper function for setting up a KvmSplitIrqChip
    fn get_split_chip() -> KvmSplitIrqChip {
        let kvm = Kvm::new().expect("failed to instantiate Kvm");
        let mem = GuestMemory::new(&[]).unwrap();
        let vm = KvmVm::new(&kvm, mem).expect("failed tso instantiate vm");

        let (_, device_socket) =
            msg_socket::pair::<VmIrqResponse, VmIrqRequest>().expect("failed to create irq socket");

        let mut chip = KvmSplitIrqChip::new(
            vm.try_clone().expect("failed to clone vm"),
            1,
            device_socket,
        )
        .expect("failed to instantiate KvmKernelIrqChip");

        let vcpu = vm.create_vcpu(0).expect("failed to instantiate vcpu");
        chip.add_vcpu(0, vcpu).expect("failed to add vcpu");
        chip
    }

    #[test]
    fn kernel_irqchip_get_pic() {
        test_get_pic(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_set_pic() {
        test_set_pic(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_get_ioapic() {
        test_get_ioapic(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_set_ioapic() {
        test_set_ioapic(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_get_pit() {
        test_get_pit(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_set_pit() {
        test_set_pit(get_kernel_chip());
    }

    #[test]
    fn kernel_irqchip_get_lapic() {
        test_get_lapic(get_kernel_chip())
    }

    #[test]
    fn kernel_irqchip_set_lapic() {
        test_set_lapic(get_kernel_chip())
    }

    #[test]
    fn kernel_irqchip_route_irq() {
        test_route_irq(get_kernel_chip());
    }

    #[test]
    fn split_irqchip_get_pic() {
        test_get_pic(get_split_chip());
    }

    #[test]
    fn split_irqchip_set_pic() {
        test_set_pic(get_split_chip());
    }

    #[test]
    fn split_irqchip_get_ioapic() {
        test_get_ioapic(get_split_chip());
    }

    #[test]
    fn split_irqchip_set_ioapic() {
        test_set_ioapic(get_split_chip());
    }

    #[test]
    fn split_irqchip_get_pit() {
        test_get_pit(get_split_chip());
    }

    #[test]
    fn split_irqchip_set_pit() {
        test_set_pit(get_split_chip());
    }

    #[test]
    fn split_irqchip_route_irq() {
        test_route_irq(get_split_chip());
    }

    #[test]
    fn split_irqchip_routes_conflict() {
        let mut chip = get_split_chip();
        chip.route_irq(IrqRoute {
            gsi: 5,
            source: IrqSource::Msi {
                address: 4276092928,
                data: 0,
            },
        })
        .expect("failed to set msi rout");
        // this second route should replace the first
        chip.route_irq(IrqRoute {
            gsi: 5,
            source: IrqSource::Msi {
                address: 4276092928,
                data: 32801,
            },
        })
        .expect("failed to set msi rout");
    }

    #[test]
    fn irq_event_tokens() {
        let mut chip = get_split_chip();
        let tokens = chip
            .irq_event_tokens()
            .expect("could not get irq_event_tokens");

        // there should be one token on a fresh split irqchip, for the pit
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, 0);

        // register another irq event
        let evt = EventFd::new().expect("failed to create eventfd");
        chip.register_irq_event(6, &evt, None)
            .expect("failed to register irq event");

        let tokens = chip
            .irq_event_tokens()
            .expect("could not get irq_event_tokens");

        // now there should be two tokens
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].0, 0);
        assert_eq!(tokens[1].0, 6);
        assert_eq!(tokens[1].1, evt);
    }

    #[test]
    fn finalize_devices() {
        let mut chip = get_split_chip();

        let mut mmio_bus = Bus::new();
        let mut io_bus = Bus::new();
        let mut resources = SystemAllocator::builder()
            .add_io_addresses(0xc000, 0x10000)
            .add_low_mmio_addresses(0, 2048)
            .add_high_mmio_addresses(2048, 4096)
            .create_allocator(5, false)
            .expect("failed to create SystemAllocator");

        // setup an event and a resample event for irq line 1
        let evt = EventFd::new().expect("failed to create eventfd");
        let mut resample_evt = EventFd::new().expect("failed to create eventfd");

        chip.register_irq_event(1, &evt, Some(&resample_evt))
            .expect("failed to register_irq_event");

        // Once we finalize devices, the pic/pit/ioapic should be attached to io and mmio busses
        chip.finalize_devices(&mut resources, &mut io_bus, &mut mmio_bus)
            .expect("failed to finalize devices");

        // Should not be able to allocate an irq < 24 now
        assert!(resources.allocate_irq().expect("failed to allocate irq") >= 24);

        // set PIT counter 2 to "SquareWaveGen"(aka 3) mode and "Both" access mode
        io_bus.write(0x43, &[0b10110110]);

        let state = chip.get_pit().expect("failed to get pit state");
        assert_eq!(state.channels[2].mode, 3);
        assert_eq!(state.channels[2].rw_mode, PitRWMode::Both);

        // ICW1 0x11: Edge trigger, cascade mode, ICW4 needed.
        // ICW2 0x08: Interrupt vector base address 0x08.
        // ICW3 0xff: Value written does not matter.
        // ICW4 0x13: Special fully nested mode, auto EOI.
        io_bus.write(0x20, &[0x11]);
        io_bus.write(0x21, &[0x08]);
        io_bus.write(0x21, &[0xff]);
        io_bus.write(0x21, &[0x13]);

        let state = chip
            .get_pic_state(PicSelect::Primary)
            .expect("failed to get pic state");

        // auto eoi and special fully nested mode should be turned on
        assert!(state.auto_eoi);
        assert!(state.special_fully_nested_mode);

        // Need to write to the irq event before servicing it
        evt.write(1).expect("failed to write to eventfd");

        // if we assert irq line one, and then get the resulting interrupt, an auto-eoi should
        // occur and cause the resample_event to be written to
        chip.service_irq_event(1).expect("failed to service irq");

        assert!(chip.interrupt_requested(0));
        assert_eq!(
            chip.get_external_interrupt(0)
                .expect("failed to get external interrupt"),
            // Vector is 9 because the interrupt vector base address is 0x08 and this is irq
            // line 1 and 8+1 = 9
            Some(0x9)
        );

        assert_eq!(
            resample_evt
                .read_timeout(std::time::Duration::from_secs(1))
                .expect("failed to read_timeout"),
            EventReadResult::Count(1)
        );

        // setup a ioapic redirection table entry 14
        let mut entry = IoapicRedirectionTableEntry::default();
        entry.set_vector(44);

        let irq_14_offset = 0x10 + 14 * 2;
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_14_offset]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(0, 32) as u32).to_ne_bytes(),
        );
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_14_offset + 1]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(32, 32) as u32).to_ne_bytes(),
        );

        let state = chip.get_ioapic_state().expect("failed to get ioapic state");

        // redirection table entry 14 should have a vector of 44
        assert_eq!(state.redirect_table[14].get_vector(), 44);
    }

    #[test]
    fn get_external_interrupt() {
        let mut chip = get_split_chip();
        assert!(!chip.interrupt_requested(0));

        chip.service_irq(0, true).expect("failed to service irq");
        assert!(chip.interrupt_requested(0));

        // Should return Some interrupt
        assert_eq!(
            chip.get_external_interrupt(0)
                .expect("failed to get external interrupt"),
            Some(0)
        );

        // interrupt is not requested twice
        assert!(!chip.interrupt_requested(0));
    }

    #[test]
    fn broadcast_eoi() {
        let mut chip = get_split_chip();

        let mut mmio_bus = Bus::new();
        let mut io_bus = Bus::new();
        let mut resources = SystemAllocator::builder()
            .add_io_addresses(0xc000, 0x10000)
            .add_low_mmio_addresses(0, 2048)
            .add_high_mmio_addresses(2048, 4096)
            .create_allocator(5, false)
            .expect("failed to create SystemAllocator");

        // setup an event and a resample event for irq line 1
        let evt = EventFd::new().expect("failed to create eventfd");
        let mut resample_evt = EventFd::new().expect("failed to create eventfd");

        chip.register_irq_event(1, &evt, Some(&resample_evt))
            .expect("failed to register_irq_event");

        // Once we finalize devices, the pic/pit/ioapic should be attached to io and mmio busses
        chip.finalize_devices(&mut resources, &mut io_bus, &mut mmio_bus)
            .expect("failed to finalize devices");

        // setup a ioapic redirection table entry 1 with a vector of 123
        let mut entry = IoapicRedirectionTableEntry::default();
        entry.set_vector(123);
        entry.set_trigger_mode(TriggerMode::Level);

        let irq_write_offset = 0x10 + 1 * 2;
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_write_offset]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(0, 32) as u32).to_ne_bytes(),
        );
        mmio_bus.write(IOAPIC_BASE_ADDRESS, &[irq_write_offset + 1]);
        mmio_bus.write(
            IOAPIC_BASE_ADDRESS + 0x10,
            &(entry.get(32, 32) as u32).to_ne_bytes(),
        );

        // Assert line 1
        chip.service_irq(1, true).expect("failed to service irq");

        // resample event should not be written to
        assert_eq!(
            resample_evt
                .read_timeout(std::time::Duration::from_millis(10))
                .expect("failed to read_timeout"),
            EventReadResult::Timeout
        );

        // irq line 1 should be asserted
        let state = chip.get_ioapic_state().expect("failed to get ioapic state");
        assert_eq!(state.current_interrupt_level_bitmap, 1 << 1);

        // Now broadcast an eoi for vector 123
        chip.broadcast_eoi(123).expect("failed to broadcast eoi");

        // irq line 1 should be deasserted
        let state = chip.get_ioapic_state().expect("failed to get ioapic state");
        assert_eq!(state.current_interrupt_level_bitmap, 0);

        // resample event should be written to by ioapic
        assert_eq!(
            resample_evt
                .read_timeout(std::time::Duration::from_millis(10))
                .expect("failed to read_timeout"),
            EventReadResult::Count(1)
        );
    }
}

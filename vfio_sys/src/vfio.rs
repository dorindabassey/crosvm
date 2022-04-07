/* automatically generated by tools/bindgen-all-the-things */

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Added by vfio_sys/bindgen.sh
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_region_info_with_cap {
    pub region_info: vfio_region_info,
    pub cap_info: __IncompleteArrayField<u8>,
}

#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    pub const fn new() -> Self {
        __IncompleteArrayField(::std::marker::PhantomData, [])
    }
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
    }
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::std::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::std::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}
impl<T> ::std::fmt::Debug for __IncompleteArrayField<T> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fmt.write_str("__IncompleteArrayField")
    }
}
pub const VFIO_API_VERSION: u32 = 0;
pub const VFIO_TYPE1_IOMMU: u32 = 1;
pub const VFIO_SPAPR_TCE_IOMMU: u32 = 2;
pub const VFIO_TYPE1v2_IOMMU: u32 = 3;
pub const VFIO_DMA_CC_IOMMU: u32 = 4;
pub const VFIO_EEH: u32 = 5;
pub const VFIO_TYPE1_NESTING_IOMMU: u32 = 6;
pub const VFIO_SPAPR_TCE_v2_IOMMU: u32 = 7;
pub const VFIO_NOIOMMU_IOMMU: u32 = 8;
pub const VFIO_TYPE: u32 = 59;
pub const VFIO_BASE: u32 = 100;
pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1;
pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 2;
pub const VFIO_DEVICE_FLAGS_RESET: u32 = 1;
pub const VFIO_DEVICE_FLAGS_PCI: u32 = 2;
pub const VFIO_DEVICE_FLAGS_PLATFORM: u32 = 4;
pub const VFIO_DEVICE_FLAGS_AMBA: u32 = 8;
pub const VFIO_DEVICE_FLAGS_CCW: u32 = 16;
pub const VFIO_DEVICE_FLAGS_AP: u32 = 32;
pub const VFIO_DEVICE_FLAGS_FSL_MC: u32 = 64;
pub const VFIO_DEVICE_FLAGS_CAPS: u32 = 128;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_BASE: u32 = 1;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_GROUP: u32 = 2;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_UTIL: u32 = 3;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_PFIP: u32 = 4;
pub const VFIO_REGION_INFO_FLAG_READ: u32 = 1;
pub const VFIO_REGION_INFO_FLAG_WRITE: u32 = 2;
pub const VFIO_REGION_INFO_FLAG_MMAP: u32 = 4;
pub const VFIO_REGION_INFO_FLAG_CAPS: u32 = 8;
pub const VFIO_REGION_INFO_CAP_SPARSE_MMAP: u32 = 1;
pub const VFIO_REGION_INFO_CAP_TYPE: u32 = 2;
pub const VFIO_REGION_TYPE_PCI_VENDOR_TYPE: u32 = 2147483648;
pub const VFIO_REGION_TYPE_PCI_VENDOR_MASK: u32 = 65535;
pub const VFIO_REGION_TYPE_GFX: u32 = 1;
pub const VFIO_REGION_TYPE_CCW: u32 = 2;
pub const VFIO_REGION_TYPE_MIGRATION: u32 = 3;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_OPREGION: u32 = 1;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_HOST_CFG: u32 = 2;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_LPC_CFG: u32 = 3;
pub const VFIO_REGION_SUBTYPE_NVIDIA_NVLINK2_RAM: u32 = 1;
pub const VFIO_REGION_SUBTYPE_IBM_NVLINK2_ATSD: u32 = 1;
pub const VFIO_REGION_SUBTYPE_GFX_EDID: u32 = 1;
pub const VFIO_DEVICE_GFX_LINK_STATE_UP: u32 = 1;
pub const VFIO_DEVICE_GFX_LINK_STATE_DOWN: u32 = 2;
pub const VFIO_REGION_SUBTYPE_CCW_ASYNC_CMD: u32 = 1;
pub const VFIO_REGION_SUBTYPE_CCW_SCHIB: u32 = 2;
pub const VFIO_REGION_SUBTYPE_CCW_CRW: u32 = 3;
pub const VFIO_REGION_SUBTYPE_MIGRATION: u32 = 1;
pub const VFIO_DEVICE_STATE_STOP: u32 = 0;
pub const VFIO_DEVICE_STATE_RUNNING: u32 = 1;
pub const VFIO_DEVICE_STATE_SAVING: u32 = 2;
pub const VFIO_DEVICE_STATE_RESUMING: u32 = 4;
pub const VFIO_DEVICE_STATE_MASK: u32 = 7;
pub const VFIO_REGION_INFO_CAP_MSIX_MAPPABLE: u32 = 3;
pub const VFIO_REGION_INFO_CAP_NVLINK2_SSATGT: u32 = 4;
pub const VFIO_REGION_INFO_CAP_NVLINK2_LNKSPD: u32 = 5;
pub const VFIO_IRQ_INFO_EVENTFD: u32 = 1;
pub const VFIO_IRQ_INFO_MASKABLE: u32 = 2;
pub const VFIO_IRQ_INFO_AUTOMASKED: u32 = 4;
pub const VFIO_IRQ_INFO_NORESIZE: u32 = 8;
pub const VFIO_IRQ_SET_DATA_NONE: u32 = 1;
pub const VFIO_IRQ_SET_DATA_BOOL: u32 = 2;
pub const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 4;
pub const VFIO_IRQ_SET_ACTION_MASK: u32 = 8;
pub const VFIO_IRQ_SET_ACTION_UNMASK: u32 = 16;
pub const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 32;
pub const VFIO_IRQ_SET_DATA_TYPE_MASK: u32 = 7;
pub const VFIO_IRQ_SET_ACTION_TYPE_MASK: u32 = 56;
pub const VFIO_GFX_PLANE_TYPE_PROBE: u32 = 1;
pub const VFIO_GFX_PLANE_TYPE_DMABUF: u32 = 2;
pub const VFIO_GFX_PLANE_TYPE_REGION: u32 = 4;
pub const VFIO_DEVICE_IOEVENTFD_8: u32 = 1;
pub const VFIO_DEVICE_IOEVENTFD_16: u32 = 2;
pub const VFIO_DEVICE_IOEVENTFD_32: u32 = 4;
pub const VFIO_DEVICE_IOEVENTFD_64: u32 = 8;
pub const VFIO_DEVICE_IOEVENTFD_SIZE_MASK: u32 = 15;
pub const VFIO_DEVICE_FEATURE_MASK: u32 = 65535;
pub const VFIO_DEVICE_FEATURE_GET: u32 = 65536;
pub const VFIO_DEVICE_FEATURE_SET: u32 = 131072;
pub const VFIO_DEVICE_FEATURE_PROBE: u32 = 262144;
pub const VFIO_DEVICE_FEATURE_PCI_VF_TOKEN: u32 = 0;
pub const VFIO_IOMMU_INFO_PGSIZES: u32 = 1;
pub const VFIO_IOMMU_INFO_CAPS: u32 = 2;
pub const VFIO_IOMMU_TYPE1_INFO_CAP_IOVA_RANGE: u32 = 1;
pub const VFIO_IOMMU_TYPE1_INFO_CAP_MIGRATION: u32 = 2;
pub const VFIO_IOMMU_TYPE1_INFO_DMA_AVAIL: u32 = 3;
pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1;
pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 2;
pub const VFIO_DMA_UNMAP_FLAG_GET_DIRTY_BITMAP: u32 = 1;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_START: u32 = 1;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_STOP: u32 = 2;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_GET_BITMAP: u32 = 4;
pub const VFIO_IOMMU_SPAPR_INFO_DDW: u32 = 1;
pub const VFIO_EEH_PE_DISABLE: u32 = 0;
pub const VFIO_EEH_PE_ENABLE: u32 = 1;
pub const VFIO_EEH_PE_UNFREEZE_IO: u32 = 2;
pub const VFIO_EEH_PE_UNFREEZE_DMA: u32 = 3;
pub const VFIO_EEH_PE_GET_STATE: u32 = 4;
pub const VFIO_EEH_PE_STATE_NORMAL: u32 = 0;
pub const VFIO_EEH_PE_STATE_RESET: u32 = 1;
pub const VFIO_EEH_PE_STATE_STOPPED: u32 = 2;
pub const VFIO_EEH_PE_STATE_STOPPED_DMA: u32 = 4;
pub const VFIO_EEH_PE_STATE_UNAVAIL: u32 = 5;
pub const VFIO_EEH_PE_RESET_DEACTIVATE: u32 = 5;
pub const VFIO_EEH_PE_RESET_HOT: u32 = 6;
pub const VFIO_EEH_PE_RESET_FUNDAMENTAL: u32 = 7;
pub const VFIO_EEH_PE_CONFIGURE: u32 = 8;
pub const VFIO_EEH_PE_INJECT_ERR: u32 = 9;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_info_cap_header {
    pub id: u16,
    pub version: u16,
    pub next: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_group_status {
    pub argsz: u32,
    pub flags: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_device_info {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
    pub cap_offset: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_info {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_sparse_mmap_area {
    pub offset: u64,
    pub size: u64,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_region_info_cap_sparse_mmap {
    pub header: vfio_info_cap_header,
    pub nr_areas: u32,
    pub reserved: u32,
    pub areas: __IncompleteArrayField<vfio_region_sparse_mmap_area>,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_info_cap_type {
    pub header: vfio_info_cap_header,
    pub type_: u32,
    pub subtype: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_gfx_edid {
    pub edid_offset: u32,
    pub edid_max_size: u32,
    pub edid_size: u32,
    pub max_xres: u32,
    pub max_yres: u32,
    pub link_state: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_device_migration_info {
    pub device_state: u32,
    pub reserved: u32,
    pub pending_bytes: u64,
    pub data_offset: u64,
    pub data_size: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_info_cap_nvlink2_ssatgt {
    pub header: vfio_info_cap_header,
    pub tgt: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_region_info_cap_nvlink2_lnkspd {
    pub header: vfio_info_cap_header,
    pub link_speed: u32,
    pub __pad: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_irq_info {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub count: u32,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_irq_set {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub start: u32,
    pub count: u32,
    pub data: __IncompleteArrayField<u8>,
}
pub const VFIO_PCI_BAR0_REGION_INDEX: ::std::os::raw::c_uint = 0;
pub const VFIO_PCI_BAR1_REGION_INDEX: ::std::os::raw::c_uint = 1;
pub const VFIO_PCI_BAR2_REGION_INDEX: ::std::os::raw::c_uint = 2;
pub const VFIO_PCI_BAR3_REGION_INDEX: ::std::os::raw::c_uint = 3;
pub const VFIO_PCI_BAR4_REGION_INDEX: ::std::os::raw::c_uint = 4;
pub const VFIO_PCI_BAR5_REGION_INDEX: ::std::os::raw::c_uint = 5;
pub const VFIO_PCI_ROM_REGION_INDEX: ::std::os::raw::c_uint = 6;
pub const VFIO_PCI_CONFIG_REGION_INDEX: ::std::os::raw::c_uint = 7;
pub const VFIO_PCI_VGA_REGION_INDEX: ::std::os::raw::c_uint = 8;
pub const VFIO_PCI_NUM_REGIONS: ::std::os::raw::c_uint = 9;
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
pub const VFIO_PCI_INTX_IRQ_INDEX: ::std::os::raw::c_uint = 0;
pub const VFIO_PCI_MSI_IRQ_INDEX: ::std::os::raw::c_uint = 1;
pub const VFIO_PCI_MSIX_IRQ_INDEX: ::std::os::raw::c_uint = 2;
pub const VFIO_PCI_ERR_IRQ_INDEX: ::std::os::raw::c_uint = 3;
pub const VFIO_PCI_REQ_IRQ_INDEX: ::std::os::raw::c_uint = 4;
pub const VFIO_PCI_NUM_IRQS: ::std::os::raw::c_uint = 5;
pub type _bindgen_ty_2 = ::std::os::raw::c_uint;
pub const VFIO_CCW_CONFIG_REGION_INDEX: ::std::os::raw::c_uint = 0;
pub const VFIO_CCW_NUM_REGIONS: ::std::os::raw::c_uint = 1;
pub type _bindgen_ty_3 = ::std::os::raw::c_uint;
pub const VFIO_CCW_IO_IRQ_INDEX: ::std::os::raw::c_uint = 0;
pub const VFIO_CCW_CRW_IRQ_INDEX: ::std::os::raw::c_uint = 1;
pub const VFIO_CCW_NUM_IRQS: ::std::os::raw::c_uint = 2;
pub type _bindgen_ty_4 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_pci_dependent_device {
    pub group_id: u32,
    pub segment: u16,
    pub bus: u8,
    pub devfn: u8,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_pci_hot_reset_info {
    pub argsz: u32,
    pub flags: u32,
    pub count: u32,
    pub devices: __IncompleteArrayField<vfio_pci_dependent_device>,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_pci_hot_reset {
    pub argsz: u32,
    pub flags: u32,
    pub count: u32,
    pub group_fds: __IncompleteArrayField<i32>,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vfio_device_gfx_plane_info {
    pub argsz: u32,
    pub flags: u32,
    pub drm_plane_type: u32,
    pub drm_format: u32,
    pub drm_format_mod: u64,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub size: u32,
    pub x_pos: u32,
    pub y_pos: u32,
    pub x_hot: u32,
    pub y_hot: u32,
    pub __bindgen_anon_1: vfio_device_gfx_plane_info__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union vfio_device_gfx_plane_info__bindgen_ty_1 {
    pub region_index: u32,
    pub dmabuf_id: u32,
}
impl Default for vfio_device_gfx_plane_info__bindgen_ty_1 {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
impl Default for vfio_device_gfx_plane_info {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_device_ioeventfd {
    pub argsz: u32,
    pub flags: u32,
    pub offset: u64,
    pub data: u64,
    pub fd: i32,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_device_feature {
    pub argsz: u32,
    pub flags: u32,
    pub data: __IncompleteArrayField<u8>,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_type1_info {
    pub argsz: u32,
    pub flags: u32,
    pub iova_pgsizes: u64,
    pub cap_offset: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iova_range {
    pub start: u64,
    pub end: u64,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_iommu_type1_info_cap_iova_range {
    pub header: vfio_info_cap_header,
    pub nr_iovas: u32,
    pub reserved: u32,
    pub iova_ranges: __IncompleteArrayField<vfio_iova_range>,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_type1_info_cap_migration {
    pub header: vfio_info_cap_header,
    pub flags: u32,
    pub pgsize_bitmap: u64,
    pub max_dirty_bitmap_size: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_type1_info_dma_avail {
    pub header: vfio_info_cap_header,
    pub avail: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_type1_dma_map {
    pub argsz: u32,
    pub flags: u32,
    pub vaddr: u64,
    pub iova: u64,
    pub size: u64,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vfio_bitmap {
    pub pgsize: u64,
    pub size: u64,
    pub data: *mut u64,
}
impl Default for vfio_bitmap {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_iommu_type1_dma_unmap {
    pub argsz: u32,
    pub flags: u32,
    pub iova: u64,
    pub size: u64,
    pub data: __IncompleteArrayField<u8>,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_iommu_type1_dirty_bitmap {
    pub argsz: u32,
    pub flags: u32,
    pub data: __IncompleteArrayField<u8>,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vfio_iommu_type1_dirty_bitmap_get {
    pub iova: u64,
    pub size: u64,
    pub bitmap: vfio_bitmap,
}
impl Default for vfio_iommu_type1_dirty_bitmap_get {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_spapr_tce_ddw_info {
    pub pgsizes: u64,
    pub max_dynamic_windows_supported: u32,
    pub levels: u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_spapr_tce_info {
    pub argsz: u32,
    pub flags: u32,
    pub dma32_window_start: u32,
    pub dma32_window_size: u32,
    pub ddw: vfio_iommu_spapr_tce_ddw_info,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_eeh_pe_err {
    pub type_: u32,
    pub func: u32,
    pub addr: u64,
    pub mask: u64,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vfio_eeh_pe_op {
    pub argsz: u32,
    pub flags: u32,
    pub op: u32,
    pub __bindgen_anon_1: vfio_eeh_pe_op__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union vfio_eeh_pe_op__bindgen_ty_1 {
    pub err: vfio_eeh_pe_err,
}
impl Default for vfio_eeh_pe_op__bindgen_ty_1 {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
impl Default for vfio_eeh_pe_op {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_spapr_register_memory {
    pub argsz: u32,
    pub flags: u32,
    pub vaddr: u64,
    pub size: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_spapr_tce_create {
    pub argsz: u32,
    pub flags: u32,
    pub page_shift: u32,
    pub __resv1: u32,
    pub window_size: u64,
    pub levels: u32,
    pub __resv2: u32,
    pub start_addr: u64,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vfio_iommu_spapr_tce_remove {
    pub argsz: u32,
    pub flags: u32,
    pub start_addr: u64,
}

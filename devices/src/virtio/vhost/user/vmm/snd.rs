// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::virtio::vhost::user::vmm::Connection;
use crate::virtio::vhost::user::vmm::Result;
use crate::virtio::vhost::user::vmm::VhostUserVirtioDevice;
use crate::virtio::DeviceType;

// control, event, tx, and rx queues
const NUM_QUEUES: usize = 4;

impl VhostUserVirtioDevice {
    pub fn new_snd(
        base_features: u64,
        connection: Connection,
        max_queue_size: Option<u16>,
    ) -> Result<VhostUserVirtioDevice> {
        let default_queues = NUM_QUEUES;

        VhostUserVirtioDevice::new(
            connection,
            DeviceType::Sound,
            default_queues,
            max_queue_size,
            base_features,
            None,
        )
    }
}

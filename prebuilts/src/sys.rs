// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "android", target_os = "linux"))] {
        pub(crate) mod unix;
        pub(crate) use unix::*;
    } else if #[cfg(windows)] {
        pub(crate) mod windows;
        pub(crate) use windows::*;
    }
}

// Copyright 2024 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Defines a struct to represent an ext2 filesystem and implements methods to create
// a filesystem in memory.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use base::MappedRegion;
use base::MemoryMappingArena;
use base::MemoryMappingBuilder;
use base::Protection;
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

use crate::arena::Arena;
use crate::arena::BlockId;
use crate::blockgroup::GroupMetaData;
use crate::blockgroup::BLOCK_SIZE;
use crate::inode::Inode;
use crate::inode::InodeBlock;
use crate::inode::InodeNum;
use crate::inode::InodeType;
use crate::superblock::Config;
use crate::superblock::SuperBlock;

#[repr(C)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes, Debug)]
struct DirEntryRaw {
    inode: u32,
    rec_len: u16,
    name_len: u8,
    file_type: u8,
}

struct DirEntryWithName<'a> {
    de: &'a mut DirEntryRaw,
    name: OsString,
}

impl<'a> std::fmt::Debug for DirEntryWithName<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirEntry")
            .field("de", &self.de)
            .field("name", &self.name)
            .finish()
    }
}

impl<'a> DirEntryWithName<'a> {
    fn new(
        arena: &'a Arena<'a>,
        inode: InodeNum,
        typ: InodeType,
        name_str: &OsStr,
        dblock: &mut DirEntryBlock,
    ) -> Result<Self> {
        if name_str.len() > 255 {
            anyhow::bail!("name length must not exceed 255: {:?}", name_str);
        }
        let cs = name_str.as_bytes();
        let name_len = cs.len();
        let aligned_name_len = name_len
            .checked_next_multiple_of(4)
            .expect("name length must be 4-byte aligned");

        // rec_len = |inode| + |file_type| + |name_len| + |rec_len| + name + padding
        //         = 4 + 1 + 1 + 2 + |name| + padding
        //         = 8 + |name| + padding
        // The padding is inserted because the name is 4-byte aligned.
        let rec_len = 8 + aligned_name_len as u16;

        let dir_entry_size = std::mem::size_of::<DirEntryRaw>();
        if dblock.offset + dir_entry_size + aligned_name_len > BLOCK_SIZE {
            bail!("sum of dir_entry size exceeds block size: {} + {dir_entry_size} + {aligned_name_len} > {BLOCK_SIZE}", dblock.offset);
        }

        let de = arena.allocate(dblock.block_id, dblock.offset)?;
        *de = DirEntryRaw {
            inode: inode.into(),
            rec_len,
            name_len: name_len as u8,
            file_type: typ.into_dir_entry_file_type(),
        };
        dblock.offset += dir_entry_size;

        let name_slice = arena.allocate_slice(dblock.block_id, dblock.offset, aligned_name_len)?;
        dblock.offset += aligned_name_len;
        name_slice[..cs.len()].copy_from_slice(cs);

        if dblock.entries.is_empty() {
            de.rec_len = BLOCK_SIZE as u16;
        } else {
            let last = dblock
                .entries
                .last_mut()
                .expect("parent_dir must not be empty");
            let last_rec_len = last.de.rec_len;
            last.de.rec_len = (8 + last.name.as_os_str().as_bytes().len() as u16)
                .checked_next_multiple_of(4)
                .expect("overflow to calculate rec_len");
            de.rec_len = last_rec_len - last.de.rec_len;
        }

        Ok(Self {
            de,
            name: name_str.into(),
        })
    }
}

#[derive(Debug)]
struct DirEntryBlock<'a> {
    block_id: BlockId,
    offset: usize,
    entries: Vec<DirEntryWithName<'a>>,
}

/// Information on how to mmap a host file to ext2 blocks.
struct FileMappingInfo {
    /// The ext2 disk block id that the memory region maps to.
    start_block: BlockId,
    /// The file to be mmap'd.
    file: File,
    /// The size of the file to be mmap'd.
    file_size: usize,
}

/// A struct to represent an ext2 filesystem.
pub struct Ext2<'a> {
    sb: &'a mut SuperBlock,

    // We support only one block group for now.
    // TODO(b/331764754): Support multiple block groups.
    group_metadata: GroupMetaData<'a>,

    // TODO(b/331901633): To support larger directory,
    // the value should be `Vec<DirEntryBlock>`.
    dentries: BTreeMap<InodeNum, DirEntryBlock<'a>>,

    fd_mappings: Vec<FileMappingInfo>,
}

impl<'a> Ext2<'a> {
    /// Create a new ext2 filesystem.
    fn new(cfg: &Config, arena: &'a Arena<'a>) -> Result<Self> {
        let sb = SuperBlock::new(arena, cfg)?;
        if sb.block_group_nr != 1 {
            bail!("multiple block group isn't supported");
        }

        let group_metadata = GroupMetaData::new(arena, sb)?;
        let mut ext2 = Ext2 {
            sb,
            group_metadata,
            dentries: BTreeMap::new(),
            fd_mappings: Vec::new(),
        };

        // Add rootdir
        let root_inode = InodeNum::new(2)?;
        ext2.add_reserved_dir(arena, root_inode, root_inode, OsStr::new("/"))?;
        let lost_found_inode = ext2.allocate_inode()?;
        ext2.add_reserved_dir(
            arena,
            lost_found_inode,
            root_inode,
            OsStr::new("lost+found"),
        )?;

        Ok(ext2)
    }

    fn block_size(&self) -> u64 {
        // Minimal block size is 1024.
        1024 << self.sb.log_block_size
    }

    fn allocate_inode(&mut self) -> Result<InodeNum> {
        if self.sb.free_inodes_count == 0 {
            bail!(
                "no free inodes: run out of s_inodes_count={}",
                self.sb.inodes_count
            );
        }

        if self.group_metadata.group_desc.free_inodes_count == 0 {
            bail!("no free inodes in group 0");
        }

        let gm = &mut self.group_metadata;
        let alloc_inode = InodeNum::new(gm.first_free_inode)?;
        // (alloc_inode - 1) because inode is 1-indexed.
        gm.inode_bitmap
            .set(usize::from(alloc_inode) - 1usize, true)?;
        gm.first_free_inode += 1;
        gm.group_desc.free_inodes_count -= 1;
        self.sb.free_inodes_count -= 1;

        Ok(alloc_inode)
    }

    fn allocate_block(&mut self) -> Result<BlockId> {
        self.allocate_contiguous_blocks(1).map(|v| v[0])
    }

    fn allocate_contiguous_blocks(&mut self, n: u16) -> Result<Vec<BlockId>> {
        if n == 0 {
            bail!("n must be positive");
        }

        if self.sb.free_blocks_count == 0 {
            bail!(
                "no free blocks: run out of s_blocks_count={}",
                self.sb.blocks_count
            );
        }

        if self.group_metadata.group_desc.free_blocks_count < n {
            // TODO(b/331764754): Support multiple block groups.
            bail!(
                "not enough free blocks in group 0.: {} < {}",
                self.group_metadata.group_desc.free_blocks_count,
                n
            );
        }

        let gm = &mut self.group_metadata;
        let alloc_blocks = (gm.first_free_block..gm.first_free_block + n as u32)
            .map(BlockId::from)
            .collect();
        gm.first_free_block += n as u32;
        gm.group_desc.free_blocks_count -= n;
        self.sb.free_blocks_count -= n as u32;
        for &b in &alloc_blocks {
            gm.block_bitmap.set(u32::from(b) as usize, true)?;
        }

        Ok(alloc_blocks)
    }

    fn get_inode_mut(&mut self, num: InodeNum) -> Result<&mut &'a mut Inode> {
        self.group_metadata
            .inode_table
            .get_mut(&num)
            .ok_or_else(|| anyhow!("{:?} not found", num))
    }

    fn allocate_dir_entry(
        &mut self,
        arena: &'a Arena<'a>,
        parent: InodeNum,
        inode: InodeNum,
        typ: InodeType,
        name: &OsStr,
    ) -> Result<()> {
        let block_size = self.block_size();

        // Disable false-positive `clippy::map_entry`.
        // https://github.com/rust-lang/rust-clippy/issues/9470
        #[allow(clippy::map_entry)]
        if !self.dentries.contains_key(&parent) {
            let block_id = self.allocate_block()?;
            let inode = self.get_inode_mut(parent)?;
            inode.block.set_block_id(0, &block_id);
            inode.blocks = block_size as u32 / 512;
            self.dentries.insert(
                parent,
                DirEntryBlock {
                    block_id,
                    offset: 0,
                    entries: Vec::new(),
                },
            );
        }

        if typ == InodeType::Directory {
            let parent = self.get_inode_mut(parent)?;
            parent.links_count += 1;
        }

        let parent_dir = self
            .dentries
            .get_mut(&parent)
            .ok_or_else(|| anyhow!("parent {:?} not found for {:?}", parent, inode))?;

        let dir_entry = DirEntryWithName::new(arena, inode, typ, name, parent_dir)?;

        parent_dir.entries.push(dir_entry);

        Ok(())
    }

    fn add_inode(&mut self, num: InodeNum, inode: &'a mut Inode) -> Result<()> {
        let typ = inode.typ().ok_or_else(|| anyhow!("unknown inode type"))?;
        if self.group_metadata.inode_table.contains_key(&num) {
            bail!("inode {:?} already exists", &num);
        }

        if typ == InodeType::Directory {
            self.group_metadata.group_desc.used_dirs_count += 1;
        }

        self.group_metadata.inode_table.insert(num, inode);

        // TODO(b/331764754): To support multiple block groups, need to fix this calculation.
        self.group_metadata
            .inode_bitmap
            .set(num.to_table_index(), true)?;

        Ok(())
    }

    // Creates a reserved directory such as "root" or "lost+found".
    // So, inode is constructed from scratch.
    fn add_reserved_dir(
        &mut self,
        arena: &'a Arena<'a>,
        inode_num: InodeNum,
        parent_inode: InodeNum,
        name: &OsStr,
    ) -> Result<()> {
        let block_size = self.sb.block_size();
        let inode = Inode::new(
            arena,
            &mut self.group_metadata,
            inode_num,
            InodeType::Directory,
            block_size as u32,
        )?;
        self.add_inode(inode_num, inode)?;

        self.allocate_dir_entry(
            arena,
            inode_num,
            inode_num,
            InodeType::Directory,
            OsStr::new("."),
        )?;
        self.allocate_dir_entry(
            arena,
            inode_num,
            parent_inode,
            InodeType::Directory,
            OsStr::new(".."),
        )?;

        if inode_num != parent_inode {
            self.allocate_dir_entry(arena, parent_inode, inode_num, InodeType::Directory, name)?;
        }

        Ok(())
    }

    fn add_dir(
        &mut self,
        arena: &'a Arena<'a>,
        inode_num: InodeNum,
        parent_inode: InodeNum,
        path: &Path,
    ) -> Result<()> {
        let block_size = self.sb.block_size();

        let inode = Inode::from_metadata(
            arena,
            &mut self.group_metadata,
            inode_num,
            &std::fs::metadata(path)?,
            block_size as u32,
            0,
            0,
            InodeBlock::default(),
        )?;

        self.add_inode(inode_num, inode)?;

        self.allocate_dir_entry(
            arena,
            inode_num,
            inode_num,
            InodeType::Directory,
            OsStr::new("."),
        )?;
        self.allocate_dir_entry(
            arena,
            inode_num,
            parent_inode,
            InodeType::Directory,
            OsStr::new(".."),
        )?;

        if inode_num != parent_inode {
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("failed to get directory name"))?;
            self.allocate_dir_entry(arena, parent_inode, inode_num, InodeType::Directory, name)?;
        }

        Ok(())
    }

    fn add_file(
        &mut self,
        arena: &'a Arena<'a>,
        parent_inode: InodeNum,
        path: &Path,
    ) -> Result<()> {
        let inode_num = self.allocate_inode()?;

        let name = path
            .file_name()
            .ok_or_else(|| anyhow!("failed to get directory name"))?;
        let file = File::open(path)?;
        let file_size = file.metadata()?.len() as usize;
        let block_size = self.block_size() as usize;
        let mut block = InodeBlock::default();

        let block_num = file_size.div_ceil(block_size);
        if block_num > 12 {
            // TODO(b/342937441): Support indirect blocks.
            bail!("indirect data block are not yet supported");
        }

        if block_num > 0 {
            let blocks = self.allocate_contiguous_blocks(block_num as u16)?;
            self.fd_mappings.push(FileMappingInfo {
                start_block: blocks[0],
                file_size,
                file,
            });
            block.copy_from_slice(0, blocks.as_bytes());
        }

        // The spec says that the `blocks` field is a "32-bit value representing the total number
        // of 512-bytes blocks". This `512` is a fixed number regardless of the actual block size,
        // which is usuaully 4KB.
        let blocks = block_num as u32 * (block_size as u32 / 512);
        let size = file_size as u32;
        let inode = Inode::from_metadata(
            arena,
            &mut self.group_metadata,
            inode_num,
            &std::fs::metadata(path)?,
            size,
            1,
            blocks,
            block,
        )?;
        self.add_inode(inode_num, inode)?;

        self.allocate_dir_entry(arena, parent_inode, inode_num, InodeType::Regular, name)?;

        Ok(())
    }

    /// Walks through `src_dir` and copies directories and files to the new file system.
    fn copy_dirtree<P: AsRef<Path>>(&mut self, arena: &'a Arena<'a>, src_dir: P) -> Result<()> {
        self.copy_dirtree_rec(arena, InodeNum(2), src_dir)
    }

    fn copy_dirtree_rec<P: AsRef<Path>>(
        &mut self,
        arena: &'a Arena<'a>,
        parent_inode: InodeNum,
        src_dir: P,
    ) -> Result<()> {
        for entry in std::fs::read_dir(src_dir)? {
            let entry = entry?;
            let ftype = entry.file_type()?;
            if ftype.is_dir() {
                let inode = self.allocate_inode()?;
                self.add_dir(arena, inode, parent_inode, &entry.path())
                    .with_context(|| {
                        format!(
                            "failed to add directory {:?} as inode={:?}",
                            entry.path(),
                            inode
                        )
                    })?;
                self.copy_dirtree_rec(arena, inode, entry.path())?;
            } else if ftype.is_file() {
                self.add_file(arena, parent_inode, &entry.path())
                    .with_context(|| {
                        format!(
                            "failed to add file {:?} in inode={:?}",
                            entry.path(),
                            parent_inode
                        )
                    })?;
            } else if ftype.is_symlink() {
                let src = entry.path();
                let dst = std::fs::read_link(&src)?;
                // TODO(b/342937495): support symlink
                bail!("symlink is not supported yet: {src:?} -> {dst:?}");
            } else {
                panic!("unknown file type: {:?}", ftype);
            }
        }

        Ok(())
    }

    fn into_fd_mappings(self) -> Vec<FileMappingInfo> {
        self.fd_mappings
    }
}

/// Creates a memory mapping region where an ext2 filesystem is constructed.
pub fn create_ext2_region(cfg: &Config, src_dir: Option<&Path>) -> Result<MemoryMappingArena> {
    let num_group = 1; // TODO(b/329359333): Support more than 1 group.
    let mut mem = MemoryMappingBuilder::new(cfg.blocks_per_group as usize * BLOCK_SIZE * num_group)
        .build()?;

    let arena = Arena::new(BLOCK_SIZE, &mut mem)?;
    let mut ext2 = Ext2::new(cfg, &arena)?;
    if let Some(dir) = src_dir {
        ext2.copy_dirtree(&arena, dir)?;
    }
    let file_mappings = ext2.into_fd_mappings();

    mem.msync()?;
    let mut mmap_arena = MemoryMappingArena::from(mem);
    for FileMappingInfo {
        start_block,
        file_size,
        file,
    } in file_mappings
    {
        mmap_arena.add_fd_mapping(
            u32::from(start_block) as usize * BLOCK_SIZE,
            file_size,
            &file,
            0, /* fd_offset */
            Protection::read(),
        )?;
    }
    Ok(mmap_arena)
}

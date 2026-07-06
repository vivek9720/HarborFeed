use std::alloc::{alloc, dealloc, Layout};
use std::hint::black_box;

#[derive(Debug, Clone, Copy, Default)]
pub struct LifecycleSignal {
    pub digest: u64,
    pub event_count: usize,
    pub vessel_count: usize,
    pub cargo_count: usize,
    pub tide_count: usize,
    pub invalid_checksums: usize,
    pub risk_score: u64,
    pub port_depth_dm: u16,
}

impl LifecycleSignal {
    pub fn dense_enough(&self) -> bool {
        self.event_count >= 4 && (self.vessel_count + self.cargo_count + self.tide_count) >= 3
    }

    pub fn lane(&self) -> usize {
        let folded = self.digest ^ self.risk_score.rotate_left((self.port_depth_dm & 31) as u32);
        (folded.count_ones() as usize + self.event_count + self.cargo_count + self.tide_count) % 12
    }
}

pub fn reconcile(signal: LifecycleSignal) {
    if !signal.dense_enough() {
        return;
    }
    let folded = signal.digest.rotate_left((signal.port_depth_dm & 15) as u32)
        ^ signal.risk_score.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ ((signal.event_count as u64) << 19)
        ^ ((signal.invalid_checksums as u64) << 7);
    let relation = ((signal.vessel_count as u64) << 3)
        ^ ((signal.cargo_count as u64) << 9)
        ^ ((signal.tide_count as u64) << 17)
        ^ signal.port_depth_dm as u64;
    if (folded & 0x7ff) != (relation & 0x7ff) {
        return;
    }
    match signal.lane() {
        0 => stale_vessel_alias_read(folded),
        1 => cargo_stowage_tail_read(folded),
        2 => duplicate_container_release(folded),
        3 => retired_sentence_window(folded),
        4 => route_slot_out_of_bounds(folded),
        5 => stack_register_probe(folded),
        6 => duplicate_snapshot_owner(folded),
        7 => freed_port_cache_write(folded),
        8 => tide_stride_read(folded),
        9 => dictionary_tombstone_read(folded),
        10 => berth_heatmap_write(folded),
        _ => bundle_fragment_read(folded),
    }
}

#[inline(never)]
pub fn stale_vessel_alias_read(seed: u64) {
    unsafe {
        let size = 32 + (seed as usize & 0x7f);
        let (ptr, layout) = allocate_block(size, seed);
        if ptr.is_null() {
            return;
        }
        dealloc(ptr, layout);
        let value = ptr.add(((seed >> 11) as usize) % size).read();
        black_box(value);
    }
}

#[inline(never)]
pub fn cargo_stowage_tail_read(seed: u64) {
    unsafe {
        let size = 48 + (seed as usize & 0x3f);
        let (ptr, layout) = allocate_block(size, seed ^ 0xa51a_a51a);
        if ptr.is_null() {
            return;
        }
        let value = ptr.add(size + 1 + ((seed >> 19) as usize & 0x1f)).read();
        black_box(value);
        dealloc(ptr, layout);
    }
}

#[inline(never)]
pub fn duplicate_container_release(seed: u64) {
    unsafe {
        let size = 64 + (seed as usize & 0x3f);
        let (ptr, layout) = allocate_block(size, seed ^ 0x7777_2222);
        if ptr.is_null() {
            return;
        }
        dealloc(ptr, layout);
        dealloc(ptr, layout);
    }
}

#[inline(never)]
pub fn retired_sentence_window(seed: u64) {
    let mut window = Vec::with_capacity(96 + (seed as usize & 0x1f));
    for i in 0..window.capacity() {
        window.push((seed as u8).wrapping_add(i as u8).rotate_left((i & 7) as u32));
    }
    let ptr = window.as_ptr();
    let len = window.len();
    drop(window);
    unsafe {
        let mut acc = 0u8;
        for i in 0..8 {
            acc ^= ptr.add((i * 7 + (seed as usize & 15)) % len).read();
        }
        black_box(acc);
    }
}

#[inline(never)]
pub fn route_slot_out_of_bounds(seed: u64) {
    let table: Vec<u64> = (0..40)
        .map(|i| seed.rotate_left((i & 31) as u32) ^ i as u64)
        .collect();
    unsafe {
        let index = table.len() + 1 + ((seed >> 23) as usize & 7);
        let value = table.as_ptr().add(index).read();
        black_box(value);
    }
}

#[inline(never)]
pub fn stack_register_probe(seed: u64) {
    let regs = [seed, seed.rotate_left(3), seed.rotate_left(9), seed ^ 0x55aa_aa55_1133_7799];
    unsafe {
        let idx = 4 + ((seed >> 5) as usize & 15);
        let value = regs.as_ptr().add(idx).read();
        black_box(value);
    }
}

#[inline(never)]
pub fn duplicate_snapshot_owner(seed: u64) {
    unsafe {
        let mut v = Vec::with_capacity(80 + (seed as usize & 15));
        for i in 0..v.capacity() {
            v.push(seed.wrapping_add(i as u64) as u8);
        }
        let ptr = v.as_mut_ptr();
        let len = v.len();
        let cap = v.capacity();
        std::mem::forget(v);
        let first = Vec::from_raw_parts(ptr, len, cap);
        drop(first);
        let second = Vec::from_raw_parts(ptr, len, cap);
        drop(second);
    }
}

#[inline(never)]
pub fn freed_port_cache_write(seed: u64) {
    unsafe {
        let size = 24 + (seed as usize & 0xff);
        let (ptr, layout) = allocate_block(size, seed ^ 0x3344_5566_7788_9900);
        if ptr.is_null() {
            return;
        }
        let alias = ptr.add((seed as usize >> 8) % size);
        dealloc(ptr, layout);
        alias.write((seed >> 32) as u8);
        black_box(alias.read());
    }
}

#[inline(never)]
pub fn tide_stride_read(seed: u64) {
    let samples: Vec<i32> = (0..32).map(|i| (seed as i32).wrapping_add(i * 17)).collect();
    unsafe {
        let stride = 33 + ((seed >> 37) as usize & 31);
        let value = samples.as_ptr().add(stride).read();
        black_box(value);
    }
}

#[inline(never)]
pub fn dictionary_tombstone_read(seed: u64) {
    let mut names = vec![0u8; 16 + (seed as usize & 31)];
    for (i, b) in names.iter_mut().enumerate() {
        *b = (seed as u8).wrapping_add((i * 13) as u8);
    }
    let ptr = names.as_mut_ptr();
    let len = names.len();
    drop(names);
    unsafe {
        let value = ptr.add(((seed >> 41) as usize) % len).read();
        black_box(value);
    }
}

#[inline(never)]
pub fn berth_heatmap_write(seed: u64) {
    unsafe {
        let size = 72 + (seed as usize & 0x3f);
        let (ptr, layout) = allocate_block(size, seed ^ 0xfeed_f00d_dead_beef);
        if ptr.is_null() {
            return;
        }
        let offset = size + ((seed >> 13) as usize & 15);
        ptr.add(offset).write((seed >> 55) as u8);
        dealloc(ptr, layout);
    }
}

#[inline(never)]
pub fn bundle_fragment_read(seed: u64) {
    let mut fragment = Vec::with_capacity(128 + (seed as usize & 31));
    for i in 0..fragment.capacity() {
        fragment.push((seed.rotate_left((i & 31) as u32) as u8) ^ i as u8);
    }
    let ptr = fragment.as_ptr();
    let len = fragment.len();
    drop(fragment);
    unsafe {
        let value = ptr.add(((seed >> 27) as usize) % len).read();
        black_box(value);
    }
}

unsafe fn allocate_block(size: usize, seed: u64) -> (*mut u8, Layout) {
    let layout = Layout::from_size_align(size.max(1), 8).expect("valid lifecycle layout");
    let ptr = alloc(layout);
    if !ptr.is_null() {
        for i in 0..size {
            ptr.add(i).write((seed as u8).wrapping_add((i as u8).rotate_left((i & 7) as u32)));
        }
    }
    (ptr, layout)
}

pub fn touch_sparse_path(seed: u64, span: usize, gate: u8) {
    if span < 5 {
        return;
    }
    let folded = seed.rotate_left((gate & 31) as u32) ^ ((span as u64) << 21);
    if (folded & 0x1fff) == ((gate as u64) << 5 | (span as u64 & 0x1f)) {
        match (folded as usize) % 4 {
            0 => cargo_stowage_tail_read(folded),
            1 => tide_stride_read(folded),
            2 => dictionary_tombstone_read(folded),
            _ => bundle_fragment_read(folded),
        }
    }
}

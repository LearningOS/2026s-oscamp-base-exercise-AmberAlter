//! # SV39 三级页表
//!
//! 本练习模拟 RISC-V SV39 三级页表的构造和地址翻译。
//! 注意，实际上的三级页表实现并非如本练习中使用 HashMap 模拟，本练习仅作为模拟帮助学习。
//! 你需要实现页表的创建、映射和地址翻译（页表遍历）。
//!
//! ## 知识点
//! - SV39：39 位虚拟地址，三级页表
//! - VPN 拆分：VPN[2] (9bit) | VPN[1] (9bit) | VPN[0] (9bit)
//! - 页表遍历（page table walk）逐级查找
//! - 大页（2MB superpage）映射
//!
//! ## SV39 虚拟地址布局
//! ```text
//! 38        30 29       21 20       12 11        0
//! ┌──────────┬───────────┬───────────┬───────────┐
//! │ VPN[2]   │  VPN[1]   │  VPN[0]   │  offset   │
//! │  9 bits  │  9 bits   │  9 bits   │  12 bits  │
//! └──────────┴───────────┴───────────┴───────────┘
//! ```

use std::collections::HashMap;

/// 页大小 4KB
pub const PAGE_SIZE: usize = 4096;
/// 每级页表有 512 个条目 (2^9)
pub const PT_ENTRIES: usize = 512;

/// PTE 标志位
pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;

/// PPN 在 PTE 中的偏移
const PPN_SHIFT: u32 = 10;

/// 页表节点：一个包含 512 个条目的数组
#[derive(Clone)]
pub struct PageTableNode {
    pub entries: [u64; PT_ENTRIES],
}

impl PageTableNode {
    pub fn new() -> Self {
        Self {
            entries: [0; PT_ENTRIES],
        }
    }
}

impl Default for PageTableNode {
    fn default() -> Self {
        Self::new()
    }
}

/// 模拟的三级页表。
///
/// 使用 HashMap<u64, PageTableNode> 模拟物理内存中的页表页。
/// `root_ppn` 是根页表所在的物理页号。
pub struct Sv39PageTable {
    /// 物理页号 -> 页表节点
    nodes: HashMap<u64, PageTableNode>,
    /// 根页表的物理页号
    pub root_ppn: u64,
    /// 下一个可分配的物理页号（简易分配器）
    next_ppn: u64,
}

/// 翻译结果
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    Ok(u64),
    PageFault,
}

impl Sv39PageTable {
    pub fn new() -> Self {
        let mut pt = Self {
            nodes: HashMap::new(),
            root_ppn: 0x80000,
            next_ppn: 0x80001,
        };
        pt.nodes.insert(pt.root_ppn, PageTableNode::new());
        pt
    }

    /// 分配一个新的物理页并初始化为空页表节点，返回其 PPN。
    fn alloc_node(&mut self) -> u64 {
        let ppn = self.next_ppn;
        self.next_ppn += 1;
        self.nodes.insert(ppn, PageTableNode::new());
        ppn
    }

    /// 从 39 位虚拟地址中提取第 `level` 级的 VPN。
    ///
    /// - level=2: 取 bits [38:30]
    /// - level=1: 取 bits [29:21]
    /// - level=0: 取 bits [20:12]
    ///
    /// 提示：右移 (12 + level * 9) 位，然后与 0x1FF 做掩码。
    pub fn extract_vpn(va: u64, level: usize) -> usize {
        // 每个 VPN 占 9 位，Offset 占 12 位
        // Level 0: 从第 12 位开始；Level 1: 第 21 位；Level 2: 第 30 位
        let shift = 12 + level * 9;
        ((va >> shift) & 0x1FF) as usize
    }

    /// 建立从虚拟页到物理页的映射（4KB 页）。
    ///
    /// 参数：
    /// - `va`: 虚拟地址（会自动对齐到页边界）
    /// - `pa`: 物理地址（会自动对齐到页边界）
    /// - `flags`: 标志位（如 PTE_V | PTE_R | PTE_W）
    pub fn map_page(&mut self, va: u64, pa: u64, flags: u64) {
        let mut current_ppn = self.root_ppn;

        // 从 Level 2 遍历到 Level 1 (中间层)
        for level in (1..=2).rev() {
            let vpn = Self::extract_vpn(va, level);
            let node = self.get_node_mut(current_ppn);
            let pte = &mut node.entries[vpn];

            if (pte.bits & PTE_V) == 0 {
                // 没路了，开辟新节点
                let new_node_ppn = self.alloc_node();
                // 注意：中间节点的 flags 只有 PTE_V
                pte.bits = (new_node_ppn << 10) | PTE_V;
            }
            current_ppn = pte.bits >> 10;
        }

        // 到达 Level 0 (叶子层)
        let vpn = Self::extract_vpn(va, 0);
        let node = self.get_node_mut(current_ppn);
        // 写入最终的物理页号和用户指定的权限标志
        node.entries[vpn].bits = (pa >> 12 << 10) | flags | PTE_V;
    }

    /// 遍历三级页表，将虚拟地址翻译为物理地址。
    ///
    /// 步骤：
    /// 1. 从根页表（root_ppn）开始
    /// 2. 对每一级（2, 1, 0）：
    ///    a. 用 VPN[level] 索引当前页表节点
    ///    b. 如果 PTE 无效（!PTE_V），返回 PageFault
    ///    c. 如果 PTE 是叶节点（R|W|X 有任一置位），提取 PPN 计算物理地址
    ///    d. 否则用 PTE 中的 PPN 进入下一级页表
    /// 3. level 0 的 PTE 必须是叶节点
    pub fn translate(&self, va: u64) -> TranslateResult {
        let mut current_ppn = self.root_ppn;

        for level in (0..=2).rev() {
            let vpn = Self::extract_vpn(va, level);
            let node = self.get_node(current_ppn);
            let pte = node.entries[vpn];

            if (pte.bits & PTE_V) == 0 {
                return TranslateResult::PageFault;
            }

            // 检查是否是叶子节点 (R, W, X 任意一个为 1)
            if (pte.bits & (PTE_R | PTE_W | PTE_X)) != 0 {
                // 物理地址 = PPN * 4KB + 虚拟地址低位的偏移
                // 注意：如果是大页，这里的偏移量会更大，但 4KB 页只需取低 12 位
                let offset_mask = (1 << (12 + level * 9)) - 1;
                let pa = (pte.bits >> 10 << 12) | (va & offset_mask);
                return TranslateResult::Ok(pa);
            }

            if level == 0 {
                // Level 0 必须是叶子，走到这说明逻辑有问题
                return TranslateResult::PageFault;
            }
            current_ppn = pte.bits >> 10;
        }
        TranslateResult::PageFault
    }

    /// 建立大页映射（2MB superpage，在 level 1 设叶子 PTE）。
    ///
    /// 2MB = 512 × 4KB，对齐要求：va 和 pa 都必须 2MB 对齐。
    ///
    /// 与 map_page 类似，但只遍历到 level 1 就写入叶子 PTE。
    pub fn map_superpage(&mut self, va: u64, pa: u64, flags: u64) {
        let mega_size: u64 = (PAGE_SIZE * PT_ENTRIES) as u64; // 2MB
        assert_eq!(va % mega_size, 0, "va must be 2MB-aligned");
        assert_eq!(pa % mega_size, 0, "pa must be 2MB-aligned");
        // 1. 从根页表开始
        let mut current_ppn = self.root_ppn;

        // 2. 处理 Level 2 (根级)
        // 我们需要通过 VPN[2] 找到 Level 1 页表的物理地址
        let vpn2 = Self::extract_vpn(va, 2);
        let node2 = self.get_node_mut(current_ppn);
        let pte2 = &mut node2.entries[vpn2];

        if (pte2.bits & PTE_V) == 0 {
            // 如果中间路径不存在，分配一个新的页表节点
            let new_node_ppn = self.alloc_node();
            // 中间节点只需设置 Valid 位，不设置 R/W/X
            pte2.bits = (new_node_ppn << 10) | PTE_V;
        }
    
        // 获取下一级（Level 1）页表的 PPN
        current_ppn = pte2.bits >> 10;

        // 3. 处理 Level 1 (目标级)
        // 关键点：在这里直接写入物理页号并设置 R/W/X 标志位，使其成为“叶子”
        let vpn1 = Self::extract_vpn(va, 1);
        let node1 = self.get_node_mut(current_ppn);
    
        // 物理页号 PPN 的计算方式和普通页一样：pa >> 12
        // 但因为它是叶子节点且位于 Level 1，硬件会自动处理剩下的 21 位偏移
        node1.entries[vpn1].bits = (pa >> 12 << 10) | flags | PTE_V;
    }
}

impl Default for Sv39PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vpn() {
        // VA = 0x0000_003F_FFFF_F000 (最大的 39 位地址的页边界)
        // VPN[2] = 0xFF (bits 38:30)
        // VPN[1] = 0x1FF (bits 29:21)
        // VPN[0] = 0x1FF (bits 20:12)
        let va: u64 = 0x7FFFFFF000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0x1FF);
    }

    #[test]
    fn test_extract_vpn_simple() {
        // VA = 0x00000000 + page 1 = 0x1000
        // VPN[2] = 0, VPN[1] = 0, VPN[0] = 1
        let va: u64 = 0x1000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 1);
    }

    #[test]
    fn test_extract_vpn_level2() {
        // VPN[2] = 1 means bit 30 set -> VA >= 0x40000000
        let va: u64 = 0x40000000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 1);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0);
    }

    #[test]
    fn test_map_and_translate_single() {
        let mut pt = Sv39PageTable::new();
        // 映射：VA 0x1000 -> PA 0x80001000
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);

        let result = pt.translate(0x1000);
        assert_eq!(result, TranslateResult::Ok(0x80001000));
    }

    #[test]
    fn test_translate_with_offset() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x2000, 0x90000000, PTE_V | PTE_R | PTE_W);

        // 访问 VA 0x2ABC -> PA 应为 0x90000ABC
        let result = pt.translate(0x2ABC);
        assert_eq!(result, TranslateResult::Ok(0x90000ABC));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = Sv39PageTable::new();
        assert_eq!(pt.translate(0x1000), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x0000_1000, 0x8000_1000, PTE_V | PTE_R);
        pt.map_page(0x0000_2000, 0x8000_5000, PTE_V | PTE_R | PTE_W);
        pt.map_page(0x0040_0000, 0x9000_0000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x1234), TranslateResult::Ok(0x80001234));
        assert_eq!(pt.translate(0x2000), TranslateResult::Ok(0x80005000));
        assert_eq!(pt.translate(0x400100), TranslateResult::Ok(0x90000100));
    }

    #[test]
    fn test_map_overwrite() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x80001000));

        pt.map_page(0x1000, 0x90002000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x90002000));
    }

    #[test]
    fn test_superpage_mapping() {
        let mut pt = Sv39PageTable::new();
        // 2MB 大页映射：VA 0x200000 -> PA 0x80200000
        pt.map_superpage(0x200000, 0x80200000, PTE_V | PTE_R | PTE_W);

        // 大页内不同偏移都应命中
        assert_eq!(pt.translate(0x200000), TranslateResult::Ok(0x80200000));
        assert_eq!(pt.translate(0x200ABC), TranslateResult::Ok(0x80200ABC));
        assert_eq!(pt.translate(0x2FF000), TranslateResult::Ok(0x802FF000));
    }

    #[test]
    fn test_superpage_and_normal_coexist() {
        let mut pt = Sv39PageTable::new();
        // 大页映射在第一个 2MB 区域
        pt.map_superpage(0x0, 0x80000000, PTE_V | PTE_R);
        // 普通页在不同的 VPN[2] 区域
        pt.map_page(0x40000000, 0x90001000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x100), TranslateResult::Ok(0x80000100));
        assert_eq!(pt.translate(0x40000000), TranslateResult::Ok(0x90001000));
    }
}

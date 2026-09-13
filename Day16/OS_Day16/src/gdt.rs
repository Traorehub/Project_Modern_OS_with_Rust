use core::ptr::addr_of_mut;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const IST_STACK_TOP: u64 = 0x380000;

static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut CS_SEL:   SegmentSelector = SegmentSelector::NULL;
static mut DATA_SEL: SegmentSelector = SegmentSelector::NULL;
static mut TSS_SEL:  SegmentSelector = SegmentSelector::NULL;

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};
    use crate::serial;

    unsafe {
        // 1. Configurer IST[0] dans la TSS
        (*addr_of_mut!(TSS))
            .interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            VirtAddr::new(IST_STACK_TOP);
        serial::println("  gdt: IST ok");

        // 2. Construire la GDT avec la crate x86_64
        let gdt = &mut *addr_of_mut!(GDT);
        let cs_sel   = gdt.append(Descriptor::kernel_code_segment());
        let data_sel = gdt.append(Descriptor::kernel_data_segment());
        let tss_sel  = gdt.append(
            Descriptor::tss_segment(&*addr_of_mut!(TSS))
        );
        *addr_of_mut!(CS_SEL)   = cs_sel;
        *addr_of_mut!(DATA_SEL) = data_sel;
        *addr_of_mut!(TSS_SEL)  = tss_sel;
        serial::println("  gdt: descripteurs ok");

        // 3. Charger la GDT
        gdt.load();
        serial::println("  gdt: LGDT ok");

        // 4. Recharger CS via far return
        let cs_val = cs_sel.0 as u64;
        core::arch::asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "rex64 retf",
            "2:",
            sel = in(reg) cs_val,
            tmp = lateout(reg) _,
        );
        serial::println("  gdt: CS ok");

        // 4bis. Charger SS, DS, ES avec le data segment
        // Obligatoire pour que iretq fonctionne correctement lors des IRQ
        use x86_64::instructions::segmentation::{SS, DS, ES, Segment};
        SS::set_reg(data_sel);
        DS::set_reg(data_sel);
        ES::set_reg(data_sel);
        serial::println("  gdt: SS/DS/ES ok");

        // 5. Charger la TSS
        load_tss(tss_sel);
        serial::println("  gdt: TSS ok !");
    }
}

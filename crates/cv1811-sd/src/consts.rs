#![allow(dead_code)]
use bit_struct::*;

pub const TOP_BASE: usize = 0xffff_ffc0_030_00000;
pub const SD_DRIVER_ADDR: usize = 0xffff_ffc0_0431_0000;
pub const SOFT_REST_BASE_ADDR: usize = 0xffff_ffc0_0300_3000;
pub const PINMUX_BASE: usize = 0xffff_ffc0_0300_1000;

pub const REG_TOP_SD_PWRSW_CTRL: usize = 0x1F4;
pub const PAD_SDIO0_CD_REG: usize = PINMUX_BASE + 0x34;
pub const PAD_SDIO0_PWR_EN_REG: usize = PINMUX_BASE + 0x38;
pub const PAD_SDIO0_CLK_REG: usize = PINMUX_BASE + 0x1C;
pub const PAD_SDIO0_CMD_REG: usize = PINMUX_BASE + 0x20;
pub const PAD_SDIO0_D0_REG: usize = PINMUX_BASE + 0x24;
pub const PAD_SDIO0_D1_REG: usize = PINMUX_BASE + 0x28;
pub const PAD_SDIO0_D2_REG: usize = PINMUX_BASE + 0x2C;
pub const PAD_SDIO0_D3_REG: usize = PINMUX_BASE + 0x30;

pub const REG_SDIO0_PAD_MASK: u32 = 0xFFFFFFF3;
pub const REG_SDIO0_PAD_SHIFT: usize = 2;
pub const REG_SDIO0_PAD_CLR_MASK: u32 = 0xC;
pub const REG_SDIO0_CD_PAD_REG: usize = PINMUX_BASE + 0x900;
pub const REG_SDIO0_CD_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_CD_PAD_RESET: u32 = 1;
pub const REG_SDIO0_PWR_EN_PAD_REG: usize = PINMUX_BASE + 0x904;
pub const REG_SDIO0_PWR_EN_PAD_VALUE: u32 = 2;
pub const REG_SDIO0_PWR_EN_PAD_RESET: u32 = 2;
pub const REG_SDIO0_CLK_PAD_REG: usize = PINMUX_BASE + 0xA00;
pub const REG_SDIO0_CLK_PAD_VALUE: u32 = 2;
pub const REG_SDIO0_CLK_PAD_RESET: u32 = 2;
pub const REG_SDIO0_CMD_PAD_REG: usize = PINMUX_BASE + 0xA04;
pub const REG_SDIO0_CMD_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_CMD_PAD_RESET: u32 = 2;
pub const REG_SDIO0_DAT0_PAD_REG: usize = PINMUX_BASE + 0xA08;
pub const REG_SDIO0_DAT0_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_DAT0_PAD_RESET: u32 = 2;
pub const REG_SDIO0_DAT1_PAD_REG: usize = PINMUX_BASE + 0xA0C;
pub const REG_SDIO0_DAT1_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_DAT1_PAD_RESET: u32 = 2;
pub const REG_SDIO0_DAT2_PAD_REG: usize = PINMUX_BASE + 0xA10;
pub const REG_SDIO0_DAT2_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_DAT2_PAD_RESET: u32 = 2;
pub const REG_SDIO0_DAT3_PAD_REG: usize = PINMUX_BASE + 0xA14;
pub const REG_SDIO0_DAT3_PAD_VALUE: u32 = 1;
pub const REG_SDIO0_DAT3_PAD_RESET: u32 = 2;

bit_struct! {
    pub struct EmmcCtrl(u32) {
        emmc_func_en: u1,
        latancy_1t: u1,
        clk_free_en: u1,
        disable_data_crc_check: u1,
        reserved: u1,
        emmc_rstn: u1,
        emmc_rstn_oen: u1,
        reserved1: u2,
        cqe_algo_sel: u1,
        cqe_prefetch_disable: u1,
        reserved2: u2,
        timer_clk_sel: u1,
        reserved3: u18
    }

    pub struct HostCtl1PwrBgWup(u32) {
        reserved_27: u5,
        wakeup_on_card_remv: u1,
        wakeup_on_card_insert: u1,
        wakeup_on_card_int: u1,
        reserved_20: u4,
        int_bg: u1,
        read_wait: u1,
        continue_req: u1,
        stop_bg_req: u1,
        reserved_12: u4,
        sd_bus_vol_sel: u3,
        sd_bus_pwr: u1,
        card_det_sel: u1,
        card_det_test: u1,
        ext_dat_width: u1,
        dma_sel: u2,
        hs_enable: u1,
        dat_xfer_width: u1,
        lec_ctl: u1,
    }

    pub struct PresentState(u32) {
        reserved_25: u7,
        cmd_line_state: u1,
        dat_3_0_state: u4,
        card_wp_state: u1,
        card_cd_state: u1,
        card_stable: u1,
        card_inserted: u1,
        reserved_12: u4,
        buf_rd_enable: u1,
        buf_wr_enable: u1,
        rd_xfer_active: u1,
        wr_xfer_active: u1,
        reserved_4: u4,
        re_tune_req: u1,
        dat_line_active: u1,
        cmd_inhibit_dat: bool,
        cmd_inhibit: bool,
    }

    pub struct SoftCpuRstn(u32) {
        reserved: u25,
        cpusys2: u1,
        cpusys1: u1,
        cpusys0: u1,
        cpucore3: u1,
        cpucore2: u1,
        cpucore1: u1,
        cpucore0: u1,
    }

    pub struct SoftCpuacRstn(u32) {
        reserved: u25,
        cpusys2: u1,
        cpusys1: u1,
        cpusys0: u1,
        cpucore3: u1,
        cpucore2: u1,
        cpucore1: u1,
        cpucore0: u1,
    }

    pub struct BlkSizeAndCnt(u32) {
        blk_cnt: u16,
        reserved: u1,
        sdma_buf_bdary: u3,
        xfer_blk_size: u12, // 0x1: 1 byte 0x2: 2 bytes ... 0x200: 512 bytes 0x800: 2048 bytes
    }

    pub struct XferModeAndCmd(u32) {
        reserved_30: u2,
        cmd_idx: u6,
        cmd_type: u2,
        data_present_sel: bool,
        cmd_idx_chk_enable: bool,
        cmd_crc_chk_enable: bool,
        sub_cmd_flag: u1,
        resp_type_sel: u2,
        reserved_9: u7,
        resp_int_enable: u1,
        resp_err_chk_enable: u1,
        resp_type: u1,
        multi_blk_sel: u1,
        dat_xfer_dir: u1,
        auto_cmd_enable: u2,
        blk_cnt_enable: bool,
        dma_enable: bool,
    }

    pub struct NormAndErrIntSts(u32) {
        reserved_29: u3,
        boot_ack_err: u1,
        reserved_27: u1,
        tune_err: u1,
        adma_err: u1,
        auto_cmd_err: u1,
        curr_limit_err: u1,
        dat_endbit_err: u1,
        dat_crc_err: u1,
        dat_tout_err: u1,
        cmd_idx_err: u1,
        cmd_endbit_err: u1,
        cmd_crc_err: u1,
        cmd_tout_err: u1,
        err_int: bool,
        cqe_event: u1,
        reserved_13: u1,
        re_tune_event: u1,
        int_c: u1,
        int_b: u1,
        int_a: u1,
        card_int: u1,
        card_remove_int: u1,
        card_insert_int: u1,
        buf_rrdy: bool,  // Buffer Read Ready
        buf_wrdy: bool,  // Buffer Write Ready
        dma_int: bool,
        bg_event: u1,
        xfer_cmpl: bool, // transfer_complete
        cmd_cmpl: bool,  // command_cmpl
    }

    pub struct ClkCtl(u32) {
        reserved27: u5,
        sw_rst_dat: u1,
        sw_rst_cmd: u1,
        sw_rst_all: u1,
        reserved20: u4,
        tout_cnt: u4,
        freq_sel: u8,
        up_freq_sel: u2,
        reserved4: u2,
        pll_en: u1,
        sd_clk_en: u1,
        int_clk_stable: u1,
        int_clk_en: u1,
    }

    pub struct AutoCmdErrAndHostCtl2(u32) {
        present_val_enable: u1,
        async_int_en: u1,
        reserved_24: u6,
        sample_clk_sel: u1,
        execute_time: u1,
        drv_sel: u2,
        en_18_sig: u1,
        uhs_mode_sel: u3,
        reserved_8: u8,
        cmd_not_issue_by_cmd12: u1,
        reserved_5: u2,
        auto_cmd_idx_err: u1,
        auto_cmd_endbit_err: u1,
        auto_cmd_crc_err: u1,
        auto_cmd_tout_err: u1,
        auto_cmd12_no_exe: u1,
    }

    pub struct Capabilities1(u32) {
        slot_type: u2,
        async_int_support: u1,
        bus64_support: u1,
        reserved_27: u1,
        v18_support: u1,
        v30_support: u1,
        v33_support: u1,
        susp_res_support: u1,
        sdma_support: u1,
        hs_support: u1,
        reserved_20: u1,
        adma2_support: u1,
        embedded_8bit: u1,
        max_blk_len: u2,
        base_clk_freq: u8,
        tout_clk_unit: u1,
        reserved_6: u1,
        tout_clk_freq: u6,
    }

    pub struct Capabilities2(u32) {
        reserved_24: u8,
        clk_multiplier: u8,
        retune_mode: u2,
        tune_sdr50: u1,
        reserved_12: u1,
        retune_timer: u4,
        reserved_7: u1,
        drv_d_support: u1,
        drv_c_support: u1,
        drv_a_support: u1,
        reserved_3: u1,
        ddr50_support: u1,
        sdr104_support: u1,
        sdr50_support: u1,
    }
}

#[derive(Debug)]
pub enum CmdError {
    IntError,
}

pub enum CommandType {
    CMD(u8),
    ACMD(u8),
}

impl CommandType {
    pub fn num(&self) -> u8 {
        match self {
            CommandType::CMD(t) => *t,
            CommandType::ACMD(t) => *t,
        }
    }
}

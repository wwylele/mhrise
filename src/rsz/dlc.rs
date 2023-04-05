use super::common::Versioned;
use super::*;
use crate::{rsz_enum, rsz_struct};
use nalgebra_glm::*;
use serde::*;

rsz_struct! {
    #[rsz("snow.data.EquipmentInventoryData.CustomBuildupResult",
        0x694F7865 = 13_00_00,
        0x6766A8A7 = 11_00_01,
    )]
    #[derive(Debug, Serialize)]
    pub struct CustomBuildupResult {
        pub id: u16, // snow.data.DataDef.CustomBuildupId
        pub value_index: u32,
        pub skill_id: PlEquipSkillId,
    }
}

rsz_struct! {
    #[rsz("snow.data.SymbolColorData",
        0x3E133A29 = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct SymbolColorData {
        pub is_enable: bool,
        pub is_default: bool,
        pub vec: IVec3,
    }
}

// snow.data.EquipmentInventoryData.IdTypes
rsz_enum! {
    #[rsz(i32)]
    #[derive(Debug, Serialize, PartialEq, Eq, Copy, Clone)]
    pub enum EquipmentInventoryDataIdTypes {
        Empty = 0,
        Weapon = 1,
        Armor = 2,
        Talisman = 3,
        LvBuffCage = 4,
    }
}

rsz_struct! {
    #[rsz("snow.data.EquipmentInventoryData",
        0x7D64033A = 14_00_00,
        0x64D86CBB = 13_00_00,
        0x52ACFD89 = 12_00_00,
        0xF90B8D8C = 11_00_01,
        0xA13D184B = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct EquipmentInventoryData {
        pub id_type: EquipmentInventoryDataIdTypes,
        pub id_val: u32,
        pub is_set_guild_card: Versioned<bool, 11_00_01>,
        pub bowgun_customize_type: i32, // snow.data.BowgunCustomize.BowgunCustomizeTypes
        pub pair_insect_inventory_index: i32,
        pub hyakuryo_skill_id_list: Vec<PlHyakuryuSkillId>,
        pub prev_hyakuryu_skill_id: Vec<PlHyakuryuSkillId>,
        pub hyakuryu_model_id: u32, // snow.data.ParamEnum.WeaponModelId
        pub hyakuryu_color_data:SymbolColorData,
        pub buildup_point: i32,
        pub is_lock: bool,
        pub talisman_deco_slot_num_list: Vec<u32>,
        pub talisman_skill_id_list: Vec<PlEquipSkillId>,
        pub talisman_skill_level_list: Vec<u32>,
        pub custom_enable: Versioned<bool, 11_00_01>,
        pub custom_count: Versioned<i32, 11_00_01>,
        pub custom_open_id_array: Versioned<Vec<u16>, 11_00_01>, // snow.data.DataDef.CustomBuildupId
        pub custom_buildup: Versioned<Vec<CustomBuildupResult>, 11_00_01>,
        pub custom_buildup_type: Versioned<i8, 13_00_00>,
        pub deco_id_list: Vec<DecorationsId>,
        pub hyakuryu_deco_id: DecorationsId,
    }
}

// snow.DlcManager.SaveLinkContents
rsz_enum! {
    #[rsz(i32)]
    #[derive(Debug, Serialize, PartialEq, Eq, Copy, Clone)]
    pub enum SaveLinkContents {
        TrialSnow = 0,
        TrialRush = 1,
        Rush = 2,
        TrialKohaku = 3,
        RushMr = 4,
        Num = 5,
        Invalid = 6,
    }
}

rsz_struct! {
    #[rsz("snow.data.Dlc.DlcAddUserData.AddDataInfo",
        0x3F93B141 = 14_00_00,
        0x9B59BC74 = 13_00_00,
        0x377D23C5 = 12_00_00,
        0xFD14730E = 11_00_01,
        0xB32786A9 = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct AddDataInfo {
        pub dlc_id: i32, // snow.DlcDef.DlcId
        pub slc_id: SaveLinkContents,
        pub pl_weapon_list: Vec<WeaponId>,
        pub pl_armor_list: Vec<PlArmorId>,
        pub pl_talisman_list: Vec<EquipmentInventoryData>,
        pub pl_overwear_id_list: Vec<PlOverwearId>,
        pub ot_overwear_id_list: Vec<OtArmorId>,
        pub pl_overwear_weapon_id_list: Versioned<Vec<u32>, 12_00_00>, // snow.data.ContentsIdSystem.OverwearWeaponId
    }
}

rsz_struct! {
    #[rsz("snow.data.Dlc.DlcAddUserData",
        path = "data/Define/DLC/DlcAddUserData.user",
        0x4C2CD0C1 = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct DlcAddUserData {
        pub add_data_info_list: Vec<AddDataInfo>,
    }
}

// snow.DlcManager.RecvType
rsz_enum! {
    #[rsz(i32)]
    #[derive(Debug, Serialize, PartialEq, Eq, Copy, Clone)]
    pub enum RecvType {
        None = 0,
        Eshop = 1,
        Bcat = 2,
        DeliveryEshop = 3,
        DeliveryBcat = 4,
    }
}

rsz_struct! {
    #[rsz("snow.DlcData",
        0xEE923A9A = 14_00_00,
        0x6883ECE4 = 13_00_00,
        0x0305CBA9 = 12_00_00,
        0x1A10B079 = 11_00_01,
        0x7397A135 = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct DlcData {
        pub dlc_id: i32, // snow.DlcDef.DlcId
        pub name: String,
        pub recv_type: RecvType,
        pub sort_category: i32, // snow.DlcManager.SortCategory
        pub title_msg_id: String,
        pub explain_msg_id: String,
        pub is_common: bool,
        pub sort_id: i32,
        pub is_need_mr: bool,
        pub need_ver: i32,
    }
}

rsz_struct! {
    #[rsz("snow.DlcListUserData",
        path = "data/Define/Common/DlcList/DlcListUserDataAsset.user",
        0xF5537B81 = 10_00_02,
    )]
    #[derive(Debug, Serialize)]
    pub struct DlcListUserData {
        pub data_list: Vec<DlcData>,
    }
}

use super::*;
use crate::{rsz_enum, rsz_newtype, rsz_struct};
use serde::*;

// snow.data.DataDef.PlArmorId
rsz_enum! {
    #[rsz(u32)]
    #[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum PlArmorId {
        TableNone = 0,
        None = 0x0C000000,
        NoneForPlEquipPack = 0x0000FFFF,
        ChangedEx = 0x00010001,
        Head(u32) = 0x0C100000..=0x0C10FFFF,
        Chest(u32) = 0x0C200000..=0x0C20FFFF,
        Arm(u32) = 0x0C300000..=0x0C30FFFF,
        Waist(u32) = 0x0C400000..=0x0C40FFFF,
        Leg(u32) = 0x0C500000..=0x0C50FFFF,
    }
}

impl PlArmorId {
    pub fn icon_index(self) -> u32 {
        match self {
            PlArmorId::Head(_) => 0,
            PlArmorId::Chest(_) => 1,
            PlArmorId::Arm(_) => 2,
            PlArmorId::Waist(_) => 3,
            PlArmorId::Leg(_) => 4,
            _ => 6,
        }
    }
}

// snow.data.DataDef.PlArmorSeriesTypes
rsz_newtype! {
    #[rsz_offset(0)]
    #[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PlArmorSeriesTypes(pub i32);
}

// snow.data.GameItemEnum.SexualEquipableFlag
rsz_enum! {
    #[rsz(i32)]
    #[derive(Debug, Serialize, Clone)]
    pub enum SexualEquipableFlag {
        MaleOnly = 0,
        FemaleOnly = 1,
        Both = 2,
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorBaseUserData.Param",
        0x68d0776c = 10_00_02
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorBaseUserDataParam {
        pub pl_armor_id: PlArmorId,
        pub is_valid: bool,
        pub series: PlArmorSeriesTypes,
        pub sort_id: u32,
        pub model_id: u32, // snow.data.DataDef.PlArmorModelId
        pub rare: RareTypes,
        pub value: u32,
        pub buy_value: u32,
        pub sexual_equipable: SexualEquipableFlag,
        pub symbol_color1: bool,
        pub symbol_color2: bool,
        pub def_val: i32,
        pub fire_reg_val: i32,
        pub water_reg_val: i32,
        pub ice_reg_val: i32,
        pub thunder_reg_val: i32,
        pub dragon_reg_val: i32,
        pub buildup_table: i32, // snow.data.ArmorBuildupData.TableTypes
        pub buff_formula: i32, // snow.data.GameItemEnum.SeriesBufType
        pub decorations_num_list: [u32; 4],
        pub skill_list: Vec<PlEquipSkillId>,
        pub skill_lv_list: Vec<i32>,
        pub id_after_ex_change: PlArmorId,
        pub cutom_table_no: u32,
        pub custom_cost: u32,
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorBaseUserData",
        path = "data/Define/Player/Armor/ArmorBaseData.user",
        0x2b3920a5 = 0
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorBaseUserData {
        pub param: Vec<ArmorBaseUserDataParam>
    }
}

// snow.data.ArmorSeriesData.EquipDifficultyGroup
rsz_enum! {
    #[rsz(i32)]
    #[derive(Debug, Serialize, Clone)]
    pub enum EquipDifficultyGroup {
        Lower = 0,
        Upper = 1,
        Master = 2,
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorSeriesUserData.Param",
        0xbf2780d1 = 10_00_02
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorSeriesUserDataParam {
        pub armor_series: PlArmorSeriesTypes,
        pub difficulty_group: EquipDifficultyGroup,
        pub is_collabo: bool,
        pub index: u32,
        pub overwear_sort_index: u32,
        pub sexual_enable: SexualEquipableFlag,
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorSeriesUserData",
        path = "data/Define/Player/Armor/ArmorSeriesData.user",
        0x45342000 = 0
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorSeriesUserData {
        pub param: Vec<ArmorSeriesUserDataParam>
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorProductUserData.Param",
        0x112ceb90 = 10_00_02
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorProductUserDataParam {
        pub id: PlArmorId,
        pub item_flag: ItemId,
        pub enemy_flag: EmTypes,
        pub progress_flag: i32, // snow.data.DataDef.UnlockProgressTypes
        pub item: Vec<ItemId>,
        pub item_num: Vec<u32>,
        pub material_category: MaterialCategory,
        pub material_category_num: u32,
        pub output_item: Vec<ItemId>,
        pub output_item_num: Vec<u32>,
    }
}

rsz_struct! {
    #[rsz("snow.data.ArmorProductUserData",
        path = "data/Define/Player/Armor/ArmorProductData.user",
        0x0b84dd13 = 0
    )]
    #[derive(Debug, Serialize)]
    pub struct ArmorProductUserData {
        pub param: Vec<ArmorProductUserDataParam>
    }
}

// snow.equip.PlOverwearId
rsz_enum! {
    #[rsz(u32)]
    #[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum PlOverwearId {
        Head(u32) = 0x0000..=0x0FFF,
        Chest(u32) = 0x1000..=0x1FFF,
        Arm(u32) = 0x2000..=0x2FFF,
        Waist(u32) = 0x3000..=0x3FFF,
        Leg(u32) = 0x4000..=0x4FFF,
    }
}

rsz_struct! {
    #[rsz("snow.equip.PlOverwearBaseUserData.Param",
        0x8be3d1fc = 10_00_02
    )]
    #[derive(Debug, Serialize)]
    pub struct PlOverwearBaseUserDataParam {
        pub id: PlOverwearId,
        pub is_valid: bool,
        pub relative_id: PlArmorId,
        pub series: PlArmorSeriesTypes,
        pub sort_id: u32,
        pub model_id: u32,
        pub rare_type: RareTypes,
        pub base_value: u32,
        pub sexual_equipable: SexualEquipableFlag,
        pub symbol_color_flg_list: Vec<bool>,
        pub is_one_set: bool,
    }
}

rsz_struct! {
    #[rsz("snow.equip.PlOverwearBaseUserData",
        path = "data/Define/Player/Armor/PlOverwearBaseData.user",
        0x495e0b7a = 0
    )]
    #[derive(Debug, Serialize)]
    pub struct PlOverwearBaseUserData {
        pub param: Vec<PlOverwearBaseUserDataParam>
    }
}

rsz_struct! {
    #[rsz("snow.equip.PlOverwearProductUserData.Param",
        0x2b8f5c1e = 10_00_02
    )]
    #[derive(Debug, Serialize)]
    pub struct PlOverwearProductUserDataParam {
        pub id: PlOverwearId,
        pub item_flag: ItemId,
        pub enemy_flag: EmTypes,
        pub progress_flag: i32, // snow.data.DataDef.UnlockProgressTypes
        pub hr_limit_flag: bool,
        pub mystery_flag: bool,
        pub item: Vec<ItemId>,
        pub item_num: Vec<u32>,
        pub material_category: MaterialCategory,
        pub material_category_num: u32,
        pub is_one_set: bool,
    }
}

rsz_struct! {
    #[rsz("snow.equip.PlOverwearProductUserData",
    path = "data/Define/Player/Armor/PlOverwearProductUserData.user",
        0x3d82ca7b = 0
    )]
    #[derive(Debug, Serialize)]
    pub struct PlOverwearProductUserData {
        pub param: Vec<PlOverwearProductUserDataParam>
    }
}

pub trait ArmorProduct {
    fn item_flag(&self) -> ItemId;
    fn enemy_flag(&self) -> EmTypes;
    fn progress_flag(&self) -> i32;
    fn item(&self) -> &Vec<ItemId>;
    fn item_num(&self) -> &Vec<u32>;
    fn material_category(&self) -> MaterialCategory;
    fn material_category_num(&self) -> u32;
}

macro_rules! impl_armor_product {
    ($name:ty) => {
        impl ArmorProduct for $name {
            fn item_flag(&self) -> ItemId {
                self.item_flag
            }
            fn enemy_flag(&self) -> EmTypes {
                self.enemy_flag
            }
            fn progress_flag(&self) -> i32 {
                self.progress_flag
            }
            fn item(&self) -> &Vec<ItemId> {
                &self.item
            }
            fn item_num(&self) -> &Vec<u32> {
                &self.item_num
            }
            fn material_category(&self) -> MaterialCategory {
                self.material_category
            }
            fn material_category_num(&self) -> u32 {
                self.material_category_num
            }
        }
    };
}

impl_armor_product!(ArmorProductUserDataParam);
impl_armor_product!(PlOverwearProductUserDataParam);

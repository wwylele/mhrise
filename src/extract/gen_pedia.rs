use super::pedia::*;
use crate::gpu::*;
use crate::gui::*;
use crate::mesh::*;
use crate::msg::*;
use crate::pak::PakReader;
use crate::pfb::Pfb;
use crate::rcol::Rcol;
use crate::rsz::*;
use crate::tex::*;
use crate::user::User;
use crate::uvs::*;
use anyhow::*;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::{HashMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::fs::*;
use std::io::{Cursor, Read, Seek, Write};
use std::ops::Deref;
use std::path::*;

fn exactly_one<T>(mut iterator: impl Iterator<Item = T>) -> Result<T> {
    let next = iterator.next().context("No element found")?;
    if iterator.next().is_some() {
        bail!("Multiple elements found");
    }
    Ok(next)
}

fn gen_em_collider_path(id: u32, sub_id: u32) -> String {
    format!(
        "enemy/em{0:03}/{1:02}/collision/em{0:03}_{1:02}_colliders.rcol",
        id, sub_id
    )
}

fn gen_ems_collider_path(id: u32, sub_id: u32) -> String {
    format!(
        "enemy/ems{0:03}/{1:02}/collision/ems{0:03}_{1:02}_colliders.rcol",
        id, sub_id
    )
}

pub fn gen_collider_mapping(rcol: Rcol) -> Result<ColliderMapping> {
    let mut meat_map: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    let mut part_map: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();

    let filter = rcol.get_monster_ride_filter();

    for attachment in rcol.group_attachments {
        if rcol.collider_groups[attachment.collider_group_index]
            .colliders
            .iter()
            .all(|collider| collider.ignore_tag_bits & filter != 0)
        {
            continue;
        }
        if let Some(data) = attachment.user_data.downcast::<EmHitDamageRsData>() {
            let entry = part_map.entry(data.parts_group.try_into()?).or_default();
            entry.insert(data.base.name.clone());
            entry.insert(attachment.name);
            entry.insert(
                rcol.collider_groups[attachment.collider_group_index]
                    .name
                    .clone(),
            );
        }
    }

    for group in rcol.collider_groups {
        for collider in group.colliders {
            if collider.ignore_tag_bits & filter != 0 {
                continue;
            }
            if let Some(data) = collider.user_data.downcast::<EmHitDamageShapeData>() {
                let entry = meat_map.entry(data.meat.try_into()?).or_default();
                entry.insert(data.base.name.clone());
            }
        }
    }

    Ok(ColliderMapping { meat_map, part_map })
}

pub fn gen_monsters(
    pak: &mut PakReader<impl Read + Seek>,
    pfb_path_gen: fn(u32, u32) -> String,
    boss_init_path_gen: fn(u32, u32) -> Option<String>,
    collider_path_gen: fn(u32, u32) -> String,
    data_tune_path_gen: fn(u32, u32) -> String,
    is_large: bool,
) -> Result<Vec<Monster>> {
    let mut monsters = vec![];

    fn sub_file<T: FromRsz + 'static>(
        pak: &mut PakReader<impl Read + Seek>,
        pfb: &Pfb,
    ) -> Result<T> {
        let path = &exactly_one(
            pfb.children
                .iter()
                .filter(|child| child.hash == T::type_hash()),
        )?
        .name;
        let index = pak.find_file(path)?;
        let data = User::new(Cursor::new(pak.read_file(index)?))?;
        data.rsz.deserialize_single().context(path.clone())
    }

    for id in 0..1000 {
        for sub_id in 0..10 {
            let main_pfb_path = pfb_path_gen(id, sub_id);
            let main_pfb_index = if let Ok(index) = pak.find_file(&main_pfb_path) {
                index
            } else {
                continue;
            };
            let main_pfb = Pfb::new(Cursor::new(pak.read_file(main_pfb_index)?))?;

            let data_base = sub_file(pak, &main_pfb).context("data_base")?;
            let data_tune = {
                // not using sub_file here because some pfb also somehow reference the variantion file
                let path = data_tune_path_gen(id, sub_id);
                let index = pak.find_file(&path)?;
                User::new(Cursor::new(pak.read_file(index)?))?
                    .rsz
                    .deserialize_single()
                    .context("data_tune")?
            };
            let meat_data = sub_file(pak, &main_pfb).context("meat_data")?;
            let condition_damage_data =
                sub_file(pak, &main_pfb).context("condition_damage_data")?;
            let anger_data = sub_file(pak, &main_pfb).context("anger_data")?;
            let parts_break_data = sub_file(pak, &main_pfb).context("parts_break_data")?;

            let boss_init_set_data = if let Some(path) = boss_init_path_gen(id, sub_id) {
                if let Ok(index) = pak.find_file(&path) {
                    let data = User::new(Cursor::new(pak.read_file(index)?))?;
                    Some(
                        data.rsz
                            .deserialize_single()
                            .context("boss_init_set_data")?,
                    )
                } else {
                    None
                }
            } else {
                None
            };

            let enemy_type = if id == 99 && sub_id == 5 {
                Some(39)
            } else {
                boss_init_set_data
                    .as_ref()
                    .map(|b: &EnemyBossInitSetData| b.enemy_type)
            };

            let rcol_path = collider_path_gen(id, sub_id);
            let rcol_index = pak.find_file(&rcol_path)?;
            let rcol =
                Rcol::new(Cursor::new(pak.read_file(rcol_index)?), true).context(rcol_path)?;
            let collider_mapping = gen_collider_mapping(rcol)?;

            let drop_item = sub_file(pak, &main_pfb).context("drop_item")?;
            let parts_break_reward = is_large
                .then(|| sub_file(pak, &main_pfb).context("parts_break_reward"))
                .transpose()?;

            monsters.push(Monster {
                id,
                sub_id,
                enemy_type,
                data_base,
                data_tune,
                meat_data,
                condition_damage_data,
                anger_data,
                parts_break_data,
                boss_init_set_data,
                collider_mapping,
                drop_item,
                parts_break_reward,
            })
        }
    }

    Ok(monsters)
}

fn get_msg(pak: &mut PakReader<impl Read + Seek>, path: &str) -> Result<Msg> {
    let index = pak.find_file(path)?;
    Msg::new(Cursor::new(pak.read_file(index)?))
}

fn get_user<T: 'static>(pak: &mut PakReader<impl Read + Seek>, path: &str) -> Result<T> {
    let index = pak.find_file(path)?;
    User::new(Cursor::new(pak.read_file(index)?))?
        .rsz
        .deserialize_single()
        .with_context(|| path.to_string())
}

fn get_weapon_list<BaseData: 'static>(
    pak: &mut PakReader<impl Read + Seek>,
    weapon_class: &str,
) -> Result<WeaponList<BaseData>> {
    Ok(WeaponList {
        base_data: get_user(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}BaseData.user",
                weapon_class
            ),
        )?,
        product: get_user(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}ProductData.user",
                weapon_class
            ),
        )?,
        change: get_user(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}ChangeData.user",
                weapon_class
            ),
        )?,
        process: get_user(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}ProcessData.user",
                weapon_class
            ),
        )?,
        tree: get_user(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}UpdateTreeData.user",
                weapon_class
            ),
        )?,
        name: get_msg(
            pak,
            &format!("data/Define/Player/Weapon/{0}/{0}_Name.msg", weapon_class),
        )?,
        explain: get_msg(
            pak,
            &format!(
                "data/Define/Player/Weapon/{0}/{0}_Explain.msg",
                weapon_class
            ),
        )?,
    })
}

pub fn gen_pedia(pak: &mut PakReader<impl Read + Seek>) -> Result<Pedia> {
    let monsters = gen_monsters(
        pak,
        |id, sub_id| {
            format!(
                "enemy/em{0:03}/{1:02}/prefab/em{0:03}_{1:02}.pfb",
                id, sub_id
            )
        },
        |id, sub_id| {
            Some(format!(
                "enemy/em{0:03}/{1:02}/user_data/em{0:03}_{1:02}_boss_init_set_data.user",
                id, sub_id
            ))
        },
        gen_em_collider_path,
        |id, sub_id| {
            format!(
                "enemy/em{0:03}/{1:02}/user_data/em{0:03}_{1:02}_datatune.user",
                id, sub_id
            )
        },
        true,
    )
    .context("Generating large monsters")?;

    let small_monsters = gen_monsters(
        pak,
        |id, sub_id| {
            format!(
                "enemy/ems{0:03}/{1:02}/prefab/ems{0:03}_{1:02}.pfb",
                id, sub_id
            )
        },
        |_, _| None,
        gen_ems_collider_path,
        |id, sub_id| {
            format!(
                "enemy/ems{0:03}/{1:02}/user_data/ems{0:03}_{1:02}_datatune.user",
                id, sub_id
            )
        },
        false,
    )
    .context("Generating small monsters")?;

    let monster_names = get_msg(pak, "Message/Tag/Tag_EM_Name.msg")?;
    let monster_aliases = get_msg(pak, "Message/Tag/Tag_EM_Name_Alias.msg")?;

    let condition_preset: EnemyConditionPresetData = get_user(
        pak,
        "enemy/user_data/system_condition_damage_preset_data.user",
    )?;
    condition_preset.verify()?;

    let monster_list = get_user(
        pak,
        "data/Define/Common/HunterNote/MonsterListBossData.user",
    )?;
    let hunter_note_msg = get_msg(pak, "Message/HunterNote/HN_Hunternote_Menu.msg")?;

    let monster_lot = get_user(
        pak,
        "data/System/RewardSystem/LotTable/MonsterLotTableData.user",
    )?;
    let parts_type = get_user(
        pak,
        "data/Define/Quest/System/QuestRewardSystem/PartsTypeTextData.user",
    )?;

    let normal_quest_data = get_user(pak, "Quest/QuestData/NormalQuestData.user")?;
    let normal_quest_data_for_enemy =
        get_user(pak, "Quest/QuestData/NormalQuestDataForEnemy.user")?;
    let dl_quest_data = get_user(pak, "Quest/QuestData/DlQuestData.user")?;
    let dl_quest_data_for_enemy = get_user(pak, "Quest/QuestData/DlQuestDataForEnemy.user")?;
    let difficulty_rate = get_user(pak, "enemy/user_data/system_difficulty_rate_data.user")?;
    let random_scale = get_user(pak, "enemy/user_data/system_boss_random_scale_data.user")?;
    let size_list = get_user(pak, "enemy/user_data/system_enemy_sizelist_data.user")?;
    let discover_em_set_data = get_user(pak, "Quest/QuestData/DiscoverEmSetData.user")?;
    let quest_hall_msg = get_msg(pak, "Message/Quest/QuestData_Hall.msg")?;
    let quest_village_msg = get_msg(pak, "Message/Quest/QuestData_Village.msg")?;
    let quest_tutorial_msg = get_msg(pak, "Message/Quest/QuestData_Tutorial.msg")?;
    let quest_arena_msg = get_msg(pak, "Message/Quest/QuestData_Arena.msg")?;
    let quest_dlc_msg = get_msg(pak, "Message/Quest/QuestData_Dlc.msg")?;

    let armor = get_user(pak, "data/Define/Player/Armor/ArmorBaseData.user")?;
    let armor_series = get_user(pak, "data/Define/Player/Armor/ArmorSeriesData.user")?;
    let armor_product = get_user(pak, "data/Define/Player/Armor/ArmorProductData.user")?;
    let overwear = get_user(pak, "data/Define/Player/Armor/PlOverwearBaseData.user")?;
    let overwear_product = get_user(
        pak,
        "data/Define/Player/Armor/PlOverwearProductUserData.user",
    )?;
    let armor_head_name_msg = get_msg(pak, "data/Define/Player/Armor/Head/A_Head_Name.msg")?;
    let armor_chest_name_msg = get_msg(pak, "data/Define/Player/Armor/Chest/A_Chest_Name.msg")?;
    let armor_arm_name_msg = get_msg(pak, "data/Define/Player/Armor/Arm/A_Arm_Name.msg")?;
    let armor_waist_name_msg = get_msg(pak, "data/Define/Player/Armor/Waist/A_Waist_Name.msg")?;
    let armor_leg_name_msg = get_msg(pak, "data/Define/Player/Armor/Leg/A_Leg_Name.msg")?;
    let armor_series_name_msg =
        get_msg(pak, "data/Define/Player/Armor/ArmorSeries_Hunter_Name.msg")?;

    let equip_skill = get_user(
        pak,
        "data/Define/Player/Skill/PlEquipSkill/PlEquipSkillBaseData.user",
    )?;
    let player_skill_detail_msg = get_msg(
        pak,
        "data/Define/Player/Skill/PlEquipSkill/PlayerSkill_Detail.msg",
    )?;
    let player_skill_explain_msg = get_msg(
        pak,
        "data/Define/Player/Skill/PlEquipSkill/PlayerSkill_Explain.msg",
    )?;
    let player_skill_name_msg = get_msg(
        pak,
        "data/Define/Player/Skill/PlEquipSkill/PlayerSkill_Name.msg",
    )?;

    let hyakuryu_skill = get_user(
        pak,
        "data/Define/Player/Skill/PlHyakuryuSkill/PlHyakuryuSkillBaseData.user",
    )?;
    let hyakuryu_skill_recipe = get_user(
        pak,
        "data/Define/Player/Skill/PlHyakuryuSkill/HyakuryuSkillRecipeData.user",
    )?;
    let hyakuryu_skill_name_msg = get_msg(
        pak,
        "data/Define/Player/Skill/PlHyakuryuSkill/HyakuryuSkill_Name.msg",
    )?;
    let hyakuryu_skill_explain_msg = get_msg(
        pak,
        "data/Define/Player/Skill/PlHyakuryuSkill/HyakuryuSkill_Explain.msg",
    )?;

    let decorations = get_user(
        pak,
        "data/Define/Player/Equip/Decorations/DecorationsBaseData.user",
    )?;

    let decorations_product = get_user(
        pak,
        "data/Define/Player/Equip/Decorations/DecorationsProductData.user",
    )?;

    let decorations_name_msg = get_msg(
        pak,
        "data/Define/Player/Equip/Decorations/Decorations_Name.msg",
    )?;

    let alchemy_pattern = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/AlchemyPatturnData.user",
    )?;
    let alchemy_pl_skill = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/AlchemyPlSkillTable.user",
    )?;
    let alchemy_grade_worth = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/GradeWorthTable.user",
    )?;
    let alchemy_rare_type = get_user(pak, "data/Define/Lobby/Facility/Alchemy/RareTypeTable.user")?;
    let alchemy_second_skill_lot = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/SecondSkillLotRateTable.user",
    )?;
    let alchemy_skill_grade_lot = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/SkillGradeLotRateTable.user",
    )?;
    let alchemy_slot_num = get_user(pak, "data/Define/Lobby/Facility/Alchemy/SlotNumTable.user")?;
    let alchemy_slot_worth = get_user(
        pak,
        "data/Define/Lobby/Facility/Alchemy/SlotWorthTable.user",
    )?;

    let items = get_user(
        pak,
        "data/System/ContentsIdSystem/Item/Normal/ItemData.user",
    )?;
    let items_name_msg = get_msg(pak, "data/System/ContentsIdSystem/Item/Normal/ItemName.msg")?;
    let items_explain_msg = get_msg(
        pak,
        "data/System/ContentsIdSystem/Item/Normal/ItemExplain.msg",
    )?;
    let material_category_msg = get_msg(
        pak,
        "data/System/ContentsIdSystem/Common/ItemCategoryType_Name.msg",
    )?;

    let great_sword = get_weapon_list(pak, "GreatSword")?;
    let short_sword = get_weapon_list(pak, "ShortSword")?;
    let hammer = get_weapon_list(pak, "Hammer")?;
    let lance = get_weapon_list(pak, "Lance")?;
    let long_sword = get_weapon_list(pak, "LongSword")?;
    let slash_axe = get_weapon_list(pak, "SlashAxe")?;
    let gun_lance = get_weapon_list(pak, "GunLance")?;
    let dual_blades = get_weapon_list(pak, "DualBlades")?;
    let horn = get_weapon_list(pak, "Horn")?;
    let insect_glaive = get_weapon_list(pak, "InsectGlaive")?;
    let charge_axe = get_weapon_list(pak, "ChargeAxe")?;
    let light_bowgun = get_weapon_list(pak, "LightBowgun")?;
    let heavy_bowgun = get_weapon_list(pak, "HeavyBowgun")?;
    let bow = get_weapon_list(pak, "Bow")?;

    let horn_melody = get_msg(pak, "data/Define/Player/Weapon/Horn/Horn_UniqueParam.msg")?;

    Ok(Pedia {
        monsters,
        small_monsters,
        monster_names,
        monster_aliases,
        condition_preset,
        monster_list,
        hunter_note_msg,
        monster_lot,
        parts_type,
        normal_quest_data,
        normal_quest_data_for_enemy,
        dl_quest_data,
        dl_quest_data_for_enemy,
        difficulty_rate,
        random_scale,
        size_list,
        discover_em_set_data,
        quest_hall_msg,
        quest_village_msg,
        quest_tutorial_msg,
        quest_arena_msg,
        quest_dlc_msg,
        armor,
        armor_series,
        armor_product,
        overwear,
        overwear_product,
        armor_head_name_msg,
        armor_chest_name_msg,
        armor_arm_name_msg,
        armor_waist_name_msg,
        armor_leg_name_msg,
        armor_series_name_msg,
        equip_skill,
        player_skill_detail_msg,
        player_skill_explain_msg,
        player_skill_name_msg,
        hyakuryu_skill,
        hyakuryu_skill_recipe,
        hyakuryu_skill_name_msg,
        hyakuryu_skill_explain_msg,
        decorations,
        decorations_product,
        decorations_name_msg,
        alchemy_pattern,
        alchemy_pl_skill,
        alchemy_grade_worth,
        alchemy_rare_type,
        alchemy_second_skill_lot,
        alchemy_skill_grade_lot,
        alchemy_slot_num,
        alchemy_slot_worth,
        items,
        items_name_msg,
        items_explain_msg,
        material_category_msg,
        great_sword,
        short_sword,
        hammer,
        lance,
        long_sword,
        slash_axe,
        gun_lance,
        dual_blades,
        horn,
        insect_glaive,
        charge_axe,
        light_bowgun,
        heavy_bowgun,
        bow,
        horn_melody,
    })
}

fn gen_monster_hitzones(
    pak: &mut PakReader<impl Read + Seek>,
    output: &Path,
    collider_path_gen: fn(u32, u32) -> String,
    mesh_path_gen: fn(u32, u32) -> String,
    meat_file_name_gen: fn(u32, u32) -> String,
    parts_group_file_name_gen: fn(u32, u32) -> String,
) -> Result<()> {
    let mut monsters = vec![];
    for index in 0..1000 {
        for sub_id in 0..10 {
            let mesh_path = mesh_path_gen(index, sub_id);
            let collider_path = collider_path_gen(index, sub_id);
            let mesh = if let Ok(mesh) = pak.find_file(&mesh_path) {
                mesh
            } else {
                continue;
            };
            let collider = if let Ok(collider) = pak.find_file(&collider_path) {
                collider
            } else {
                continue;
            };
            let mesh = pak.read_file(mesh)?;
            let collider = pak.read_file(collider)?;
            monsters.push((index, sub_id, mesh, collider));
        }
    }

    monsters
        .into_par_iter()
        .map(|(index, sub_id, mesh, collider)| {
            let mesh = Mesh::new(Cursor::new(mesh))?;
            let mut collider = Rcol::new(Cursor::new(collider), true)?;
            let meat_path = output.join(meat_file_name_gen(index, sub_id));
            let parts_group_path = output.join(parts_group_file_name_gen(index, sub_id));
            collider.apply_skeleton(&mesh)?;
            let (vertexs, indexs) = collider.color_monster_model(&mesh)?;
            let HitzoneDiagram { meat, parts_group } = gen_hitzone_diagram(vertexs, indexs)?;
            meat.save_png(&meat_path)?;
            parts_group.save_png(&parts_group_path)?;
            Ok(())
        })
        .collect::<Result<Vec<()>>>()?;

    Ok(())
}

// Monsters in title updates have icon files with special names. It is hard to
// get the name mapping without hard-coding it here.
// Icon file names, including normal ones and special ones, are referred in
// gui/01_Common/boss_icon.gui, but they have their own order in the frame
// sequence there. The mapping from EM ID to frame ID is probably done by
// snow.gui.SnowGuiCommonUtility.Icon.getEnemyIconFrame, which would be
// hard-coded in game code.
static EM_ICON_MAP: Lazy<HashMap<(i32, i32), &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // This is not applicable in PC update anymore
    /*
    m.insert((24, 0), "A0");
    m.insert((25, 0), "B1");
    m.insert((27, 0), "C2");
    m.insert((86, 5), "D3");
    m.insert((118, 0), "E4");
    // F5?
    m.insert((2, 7), "G6");
    m.insert((7, 7), "H7");
    m.insert((57, 7), "I8");
    // J9?
    // KA?
    m.insert((99, 5), "LB");
    */

    // Except... they did a oopsie on pc
    m.insert((86, 5), "em086_00");
    m
});

pub fn gen_resources(pak: &mut PakReader<impl Read + Seek>, output: &Path) -> Result<()> {
    let root = PathBuf::from(output);
    if root.exists() {
        remove_dir_all(&root)?;
    }
    create_dir(&root)?;

    let mesh_path_gen = |id, mut sub_id| {
        if id == 99 && sub_id == 5 {
            sub_id = 0;
        }
        format!("enemy/em{0:03}/{1:02}/mod/em{0:03}_{1:02}.mesh", id, sub_id)
    };

    gen_monster_hitzones(
        pak,
        &root,
        gen_em_collider_path,
        mesh_path_gen,
        |id, sub_id| format!("em{0:03}_{1:02}_meat.png", id, sub_id),
        |id, sub_id| format!("em{0:03}_{1:02}_parts_group.png", id, sub_id),
    )?;

    gen_monster_hitzones(
        pak,
        &root,
        gen_ems_collider_path,
        |id, sub_id| {
            format!(
                "enemy/ems{0:03}/{1:02}/mod/ems{0:03}_{1:02}.mesh",
                id, sub_id
            )
        },
        |id, sub_id| format!("ems{0:03}_{1:02}_meat.png", id, sub_id),
        |id, sub_id| format!("ems{0:03}_{1:02}_parts_group.png", id, sub_id),
    )?;

    for index in 0..1000 {
        for sub_id in 0..10 {
            let icon_path = if let Some(name) = EM_ICON_MAP.get(&(index, sub_id)) {
                format!("gui/80_Texture/boss_icon/{}_IAM.tex", name)
            } else {
                format!(
                    "gui/80_Texture/boss_icon/em{:03}_{1:02}_IAM.tex",
                    index, sub_id
                )
            };
            let icon = if let Ok(icon) = pak.find_file(&icon_path) {
                icon
            } else {
                continue;
            };
            let icon = Tex::new(Cursor::new(pak.read_file(icon)?))?;
            icon.save_png(
                0,
                0,
                &root.join(format!("em{0:03}_{1:02}_icon.png", index, sub_id)),
            )?;
        }
    }

    for index in 0..1000 {
        for sub_id in 0..10 {
            let icon_path = format!(
                "gui/80_Texture/boss_icon/ems{:03}_{1:02}_IAM.tex",
                index, sub_id
            );
            let icon = if let Ok(icon) = pak.find_file(&icon_path) {
                icon
            } else {
                continue;
            };
            let icon = Tex::new(Cursor::new(pak.read_file(icon)?))?;
            icon.save_png(
                0,
                0,
                &root.join(format!("ems{0:03}_{1:02}_icon.png", index, sub_id)),
            )?;
        }
    }

    let guild_card = pak.find_file("gui/80_Texture/GuildCard_IAM.tex")?;
    let guild_card = Tex::new(Cursor::new(pak.read_file(guild_card)?))?.to_rgba(0, 0)?;

    guild_card
        .sub_image(302, 397, 24, 24)?
        .save_png(&root.join("king_crown.png"))?;

    guild_card
        .sub_image(302, 453, 24, 24)?
        .save_png(&root.join("small_crown.png"))?;

    let item_icon_path = root.join("item");
    create_dir(&item_icon_path)?;
    let item_icon_uvs = pak.find_file("gui/70_UVSequence/cmn_icon.uvs")?;
    let item_icon_uvs = Uvs::new(Cursor::new(pak.read_file(item_icon_uvs)?))?;
    if item_icon_uvs.textures.len() != 1 || item_icon_uvs.spriter_groups.len() != 1 {
        bail!("Broken cmn_icon.uvs");
    }
    let item_icon = pak.find_file(&item_icon_uvs.textures[0].path)?;
    let item_icon = Tex::new(Cursor::new(pak.read_file(item_icon)?))?.to_rgba(0, 0)?;
    for (i, spriter) in item_icon_uvs.spriter_groups[0].spriters.iter().enumerate() {
        let (item_icon_r, item_icon_a) = item_icon
            .sub_image_f(spriter.p0, spriter.p1)?
            .gen_double_mask();
        item_icon_r.save_png(&item_icon_path.join(format!("{:03}.r.png", i)))?;
        item_icon_a.save_png(&item_icon_path.join(format!("{:03}.a.png", i)))?;
    }

    let item_addon_uvs = pak.find_file("gui/70_UVSequence/Item_addonicon.uvs")?;
    let item_addon_uvs = Uvs::new(Cursor::new(pak.read_file(item_addon_uvs)?))?;
    if item_addon_uvs.textures.len() != 1 || item_addon_uvs.spriter_groups.len() != 1 {
        bail!("Broken item_addon.uvs");
    }
    let item_addon = pak.find_file(&item_addon_uvs.textures[0].path)?;
    let item_addon = Tex::new(Cursor::new(pak.read_file(item_addon)?))?.to_rgba(0, 0)?;
    for (i, spriter) in item_addon_uvs.spriter_groups[0].spriters.iter().enumerate() {
        item_addon
            .sub_image_f(spriter.p0, spriter.p1)?
            .save_png(&root.join(format!("item_addon_{}.png", i)))?;
    }

    let message_window_uvs = pak.find_file("gui/70_UVSequence/message_window.uvs")?;
    let message_window_uvs = Uvs::new(Cursor::new(pak.read_file(message_window_uvs)?))?;
    if message_window_uvs.textures.len() != 1 || message_window_uvs.spriter_groups.len() != 1 {
        bail!("Broken message_window.uvs");
    }
    let message_window = pak.find_file(&message_window_uvs.textures[0].path)?;
    let message_window = Tex::new(Cursor::new(pak.read_file(message_window)?))?.to_rgba(0, 0)?;
    let skill_icon = message_window_uvs.spriter_groups[0]
        .spriters
        .get(170)
        .context("Skill icon not found")?;
    let (skill_r, skill_a) = message_window
        .sub_image_f(skill_icon.p0, skill_icon.p1)?
        .gen_double_mask();
    skill_r.save_png(&root.join("skill.r.png"))?;
    skill_a.save_png(&root.join("skill.a.png"))?;

    let equip_icon_path = root.join("equip");
    create_dir(&equip_icon_path)?;
    let equip_icon_uvs = pak.find_file("gui/70_UVSequence/EquipIcon.uvs")?;
    let equip_icon_uvs = Uvs::new(Cursor::new(pak.read_file(equip_icon_uvs)?))?;
    if equip_icon_uvs.textures.len() != 2 || equip_icon_uvs.spriter_groups.len() != 2 {
        bail!("Broken EquipIcon.uvs");
    }
    let equip_icon = pak.find_file(&equip_icon_uvs.textures[0].path)?;
    let equip_icon = Tex::new(Cursor::new(pak.read_file(equip_icon)?))?.to_rgba(0, 0)?;
    for (i, spriter) in equip_icon_uvs.spriter_groups[0].spriters.iter().enumerate() {
        let (equip_icon_r, equip_icon_a) = equip_icon
            .sub_image_f(spriter.p0, spriter.p1)?
            .gen_double_mask();
        equip_icon_r.save_png(&equip_icon_path.join(format!("{:03}.r.png", i)))?;
        equip_icon_a.save_png(&equip_icon_path.join(format!("{:03}.a.png", i)))?;
    }

    let common_uvs = pak.find_file("gui/70_UVSequence/common.uvs")?;
    let common_uvs = Uvs::new(Cursor::new(pak.read_file(common_uvs)?))?;
    if common_uvs.textures.len() != 1 || common_uvs.spriter_groups.len() != 2 {
        bail!("Broken common.uvs");
    }
    let common = pak.find_file(&common_uvs.textures[0].path)?;
    let common = Tex::new(Cursor::new(pak.read_file(common)?))?.to_rgba(0, 0)?;
    for (i, spriter) in common_uvs.spriter_groups[1].spriters.iter().enumerate() {
        common
            .sub_image_f(spriter.p0, spriter.p1)?
            .save_png(&root.join(format!("questtype_{}.png", i)))?;
    }

    let common_uvs = pak.find_file("gui/70_UVSequence/Slot_Icon.uvs")?;
    let common_uvs = Uvs::new(Cursor::new(pak.read_file(common_uvs)?))?;
    if common_uvs.textures.len() != 1 || common_uvs.spriter_groups.len() != 1 {
        bail!("Broken Slot_Icon.uvs");
    }
    let common = pak.find_file(&common_uvs.textures[0].path)?;
    let common = Tex::new(Cursor::new(pak.read_file(common)?))?.to_rgba(0, 0)?;
    for (i, spriter) in common_uvs.spriter_groups[0].spriters.iter().enumerate() {
        common
            .sub_image_f(spriter.p0, spriter.p1)?
            .save_png(&root.join(format!("slot_{}.png", i)))?;
    }

    let item_colors_path = root.join("item_color.css");
    gen_item_colors(pak, &item_colors_path)?;

    let item_colors_path = root.join("rarity_color.css");
    gen_rarity_colors(pak, &item_colors_path)?;

    Ok(())
}

fn gen_gui_colors(
    pak: &mut PakReader<impl Read + Seek>,
    output: &Path,
    gui: &str,
    control_name: &str,
    prefix: &str,
    css_prefix: &str,
) -> Result<()> {
    let mut file = File::create(output)?;
    let item_icon_gui = pak.find_file(gui)?;
    let item_icon_gui = Gui::new(Cursor::new(pak.read_file(item_icon_gui)?))?;
    let item_icon_color = item_icon_gui
        .controls
        .iter()
        .find(|control| control.name == control_name)
        .context("Control not found")?;

    fn color_tran(value: u64) -> Result<u8> {
        let value = f64::from_bits(value);
        if !(0.0..=1.0).contains(&value) {
            bail!("Bad color value");
        }
        Ok((value * 255.0).round() as u8)
    }

    for clips in &item_icon_color.clips {
        if !clips.name.starts_with(prefix) {
            continue;
        }
        let id: u32 = clips.name[prefix.len()..].parse()?;
        if clips.variable_values.len() != 3 {
            bail!("Unexpected variable values len");
        }
        let r = color_tran(clips.variable_values[0].value)?;
        let g = color_tran(clips.variable_values[1].value)?;
        let b = color_tran(clips.variable_values[2].value)?;

        writeln!(
            file,
            ".{}{} {{background-color: #{:02X}{:02X}{:02X}}}",
            css_prefix, id, r, g, b
        )?;
    }

    Ok(())
}

fn gen_item_colors(pak: &mut PakReader<impl Read + Seek>, output: &Path) -> Result<()> {
    gen_gui_colors(
        pak,
        output,
        "gui/01_Common/ItemIcon.gui",
        "pnl_ItemIcon_Color",
        "ITEM_ICON_COLOR_",
        "mh-item-color-",
    )
}

fn gen_rarity_colors(pak: &mut PakReader<impl Read + Seek>, output: &Path) -> Result<()> {
    gen_gui_colors(
        pak,
        output,
        "gui/01_Common/EquipIcon.gui",
        "pnl_EquipIcon_Color",
        "rare_col",
        "mh-rarity-color-",
    )
}

fn prepare_size_map(size_data: &EnemySizeListData) -> Result<HashMap<EmTypes, &SizeInfo>> {
    let mut result = HashMap::new();
    for size_info in &size_data.size_info_list {
        if result.insert(size_info.em_type, size_info).is_some() {
            bail!("Duplicate size info for {:?}", size_info.em_type);
        }
    }
    Ok(result)
}

fn prepare_size_dist_map(
    size_dist_data: &EnemyBossRandomScaleData,
) -> Result<HashMap<i32, &[ScaleAndRateData]>> {
    let mut result = HashMap::new();
    for size_info in &size_dist_data.random_scale_table_data_list {
        if result
            .insert(size_info.type_, &size_info.scale_and_rate_data[..])
            .is_some()
        {
            bail!("Duplicate size dist for {}", size_info.type_);
        }
    }
    if result.contains_key(&0) {
        bail!("Defined size dist for 0");
    }
    result.insert(
        0,
        &[ScaleAndRateData {
            scale: 1.0,
            rate: 100,
        }],
    );
    Ok(result)
}

fn prepare_quests(pedia: &Pedia) -> Result<Vec<Quest<'_>>> {
    let mut all_msg: HashMap<&String, &MsgEntry> = pedia
        .quest_hall_msg
        .entries
        .iter()
        .map(|entry| (&entry.name, entry))
        .chain(
            pedia
                .quest_village_msg
                .entries
                .iter()
                .map(|entry| (&entry.name, entry)),
        )
        .chain(
            pedia
                .quest_tutorial_msg
                .entries
                .iter()
                .map(|entry| (&entry.name, entry)),
        )
        .chain(
            pedia
                .quest_arena_msg
                .entries
                .iter()
                .map(|entry| (&entry.name, entry)),
        )
        .chain(
            pedia
                .quest_dlc_msg
                .entries
                .iter()
                .map(|entry| (&entry.name, entry)),
        )
        .collect();

    let mut enemy_params: HashMap<i32, &NormalQuestDataForEnemyParam> = pedia
        .normal_quest_data_for_enemy
        .param
        .iter()
        .map(|param| (param.quest_no, param))
        .chain(
            pedia
                .dl_quest_data_for_enemy
                .param
                .iter()
                .map(|param| (param.quest_no, param)),
        )
        .collect();

    pedia
        .normal_quest_data
        .param
        .iter()
        .filter(|param| param.quest_no != 0)
        .map(|param| (param, false))
        .chain(
            pedia
                .dl_quest_data
                .param
                .iter()
                .filter(|param| param.quest_no != 0)
                .map(|param| (param, true)),
        )
        .map(|(param, is_dl)| {
            let name_msg_name = format!("QN{:06}_01", param.quest_no);
            let requester_msg_name = format!("QN{:06}_02", param.quest_no);
            let detail_msg_name = format!("QN{:06}_03", param.quest_no);
            let target_msg_name = format!("QN{:06}_04", param.quest_no);
            let condition_msg_name = format!("QN{:06}_05", param.quest_no);
            Ok(Quest {
                param,
                enemy_param: enemy_params.remove(&param.quest_no),
                name: all_msg.remove(&name_msg_name),
                requester: all_msg.remove(&requester_msg_name),
                detail: all_msg.remove(&detail_msg_name),
                target: all_msg.remove(&target_msg_name),
                condition: all_msg.remove(&condition_msg_name),
                is_dl,
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn prepare_discoveries(pedia: &Pedia) -> Result<HashMap<EmTypes, &DiscoverEmSetDataParam>> {
    let mut result = HashMap::new();
    for discovery in &pedia.discover_em_set_data.param {
        if discovery.em_type == EmTypes::Em(0) {
            continue;
        }
        ensure!(discovery.param.route_no.len() == 5);
        ensure!(discovery.param.init_set_name.len() == 5);
        ensure!(discovery.param.sub_type.len() == 3);
        ensure!(discovery.param.vital_tbl.len() == 3);
        ensure!(discovery.param.attack_tbl.len() == 3);
        ensure!(discovery.param.parts_tbl.len() == 3);
        ensure!(discovery.param.other_tbl.len() == 3);
        ensure!(discovery.param.stamina_tbl.len() == 3);
        ensure!(discovery.param.scale.len() == 3);
        ensure!(discovery.param.scale_tbl.len() == 3);
        ensure!(discovery.param.difficulty.len() == 3);
        ensure!(discovery.param.boss_multi.len() == 3);

        if result.insert(discovery.em_type, discovery).is_some() {
            bail!("Duplicated discovery data for {:?}", discovery.em_type)
        }
    }

    Ok(result)
}

fn prepare_skills(pedia: &Pedia) -> Result<BTreeMap<PlEquipSkillId, Skill<'_>>> {
    let mut result = BTreeMap::new();

    let mut name_msg: HashMap<&String, &MsgEntry> = pedia.player_skill_name_msg.get_name_map();

    let mut explain_msg: HashMap<&String, &MsgEntry> =
        pedia.player_skill_explain_msg.get_name_map();

    let mut detail_msg: HashMap<&String, &MsgEntry> = pedia.player_skill_detail_msg.get_name_map();

    for skill in &pedia.equip_skill.param {
        let id = match skill.id {
            PlEquipSkillId::None => continue,
            PlEquipSkillId::Skill(id) => id,
        };
        if result.contains_key(&skill.id) {
            bail!("Multiple definition for skill {}", id);
        }

        let name = name_msg
            .remove(&format!("PlayerSkill_{:03}_Name", id))
            .with_context(|| format!("Name for skill {}", id))?;

        let explain = explain_msg
            .remove(&format!("PlayerSkill_{:03}_Explain", id))
            .with_context(|| format!("Explain for skill {}", id))?;

        let levels = (0..(skill.max_level + 1))
            .map(|level| {
                detail_msg
                    .remove(&format!("PlayerSkill_{:03}_{:02}_Detail", id, level))
                    .with_context(|| format!("Detail for skill {} level {}", id, level))
            })
            .collect::<Result<Vec<_>>>()?;

        result.insert(
            skill.id,
            Skill {
                name,
                explain,
                levels,
                icon_color: skill.icon_color,
                deco: None,
            },
        );
    }

    let mut deco_name_msg = pedia.decorations_name_msg.get_name_map();
    let mut deco_product = HashMap::new();
    for product in &pedia.decorations_product.param {
        if product.id == DecorationsId::None {
            continue;
        }
        if deco_product.insert(product.id, product).is_some() {
            bail!("Multiple product for deco {:?}", product.id)
        }
    }

    for deco in &pedia.decorations.param {
        let inner_id = if let DecorationsId::Deco(id) = deco.id {
            id
        } else {
            continue;
        };
        let product = deco_product
            .remove(&deco.id)
            .with_context(|| format!("Product not found for deco {:?}", deco.id))?;
        let name = deco_name_msg
            .remove(&format!("Decorations_{:03}_Name", inner_id))
            .with_context(|| format!("Name not found for deco {:?}", deco.id))?;
        let deco_pack = Deco {
            data: deco,
            product,
            name,
        };

        if deco.skill_id_list.len() != 1 || deco.skill_lv_list.len() != 1 {
            bail!("Multi skill deco {:?}", deco.id)
        }
        if deco.skill_lv_list[0] != 1 {
            bail!("Multi level deco {:?}", deco.id)
        }

        result
            .get_mut(&deco.skill_id_list[0])
            .with_context(|| format!("Skill not found for deco {:?}", deco.id))?
            .deco = Some(deco_pack);
    }

    Ok(result)
}

fn prepare_hyakuryu_skills(
    pedia: &Pedia,
) -> Result<BTreeMap<PlHyakuryuSkillId, HyakuryuSkill<'_>>> {
    let mut names = pedia.hyakuryu_skill_name_msg.get_name_map();
    let mut explains = pedia.hyakuryu_skill_explain_msg.get_name_map();
    let mut result = BTreeMap::new();
    let mut recipes = HashMap::new();
    for recipe in &pedia.hyakuryu_skill_recipe.param {
        if recipe.skill_id == PlHyakuryuSkillId::None {
            continue;
        }
        if recipes.insert(recipe.skill_id, recipe).is_some() {
            bail!("Multiple recipe for hyakuryu skill {:?}", recipe.skill_id);
        }
    }
    for skill in &pedia.hyakuryu_skill.param {
        let raw_id = if let PlHyakuryuSkillId::Skill(id) = skill.id {
            id
        } else {
            continue;
        };
        let recipe = recipes.remove(&skill.id);
        let name = names
            .remove(&format!("HyakuryuSkill_{:03}_Name", raw_id))
            .with_context(|| format!("No name found for hyakuryu skill {:?}", skill.id))?;
        let explain = explains
            .remove(&format!("HyakuryuSkill_{:03}_Explain", raw_id))
            .with_context(|| format!("No explain found for hyakuryu skill {:?}", skill.id))?;
        let skill_package = HyakuryuSkill {
            data: skill,
            recipe,
            name,
            explain,
        };
        if result.insert(skill.id, skill_package).is_some() {
            bail!("Multiple definition for hyakuryu skill {:?}", skill.id);
        }
    }

    Ok(result)
}

fn prepare_armors(pedia: &Pedia) -> Result<Vec<ArmorSeries<'_>>> {
    let mut product_map: HashMap<PlArmorId, &ArmorProductUserDataParam> = HashMap::new();
    for product in &pedia.armor_product.param {
        if product_map.insert(product.id, product).is_some() {
            bail!("Multiple definition for armor product {:?}", product.id);
        }
    }

    let mut series_map: BTreeMap<PlArmorSeriesTypes, ArmorSeries> = BTreeMap::new();

    for armor_series in &pedia.armor_series.param {
        if series_map.contains_key(&armor_series.armor_series) {
            bail!(
                "Duplicate armor series for ID {:?}",
                armor_series.armor_series
            );
        }
        let name = pedia
            .armor_series_name_msg
            .entries
            .get(armor_series.armor_series.0 as usize); // ?!
        let series = ArmorSeries {
            name,
            series: armor_series,
            pieces: [None, None, None, None, None, None, None, None, None, None],
        };
        series_map.insert(armor_series.armor_series, series);
    }

    for armor in &pedia.armor.param {
        if !armor.is_valid {
            continue;
        }

        let (mut slot, msg, id) = match armor.pl_armor_id {
            PlArmorId::Head(id) => (0, &pedia.armor_head_name_msg, id),
            PlArmorId::Chest(id) => (1, &pedia.armor_chest_name_msg, id),
            PlArmorId::Arm(id) => (2, &pedia.armor_arm_name_msg, id),
            PlArmorId::Waist(id) => (3, &pedia.armor_waist_name_msg, id),
            PlArmorId::Leg(id) => (4, &pedia.armor_leg_name_msg, id),
            _ => bail!("Unknown armor ID {:?}", armor.pl_armor_id),
        };

        if armor.id_after_ex_change == PlArmorId::ChangedEx {
            slot += 5;
        }

        let id = usize::try_from(id)?;

        let name = msg
            .entries
            .get(id)
            .with_context(|| format!("Cannot find name for armor {:?}", armor.pl_armor_id))?; // ?!

        let product = product_map.remove(&armor.pl_armor_id);

        let series = series_map.get_mut(&armor.series).with_context(|| {
            format!(
                "Cannot find series {:?} for armor {:?}",
                armor.series, armor.pl_armor_id
            )
        })?;

        if series.pieces[slot].is_some() {
            bail!(
                "Duplicated pieces for series {:?} slot {}",
                armor.series,
                slot
            );
        }

        series.pieces[slot] = Some(Armor {
            name,
            data: armor,
            product,
            overwear: None,
            overwear_product: None,
        });
    }

    let mut overwear_product_map = HashMap::new();
    for overwear_product in &pedia.overwear_product.param {
        if matches!(
            overwear_product.id,
            PlOverwearId::Head(0)
                | PlOverwearId::Chest(0)
                | PlOverwearId::Arm(0)
                | PlOverwearId::Waist(0)
                | PlOverwearId::Leg(0)
        ) {
            continue;
        }

        if overwear_product_map
            .insert(overwear_product.id, overwear_product)
            .is_some()
        {
            bail!(
                "Multiple definition for overwear product for {:?}",
                overwear_product.id
            );
        }
    }

    let mut overwear_set = HashSet::new();
    for overwear in &pedia.overwear.param {
        if !overwear.is_valid {
            continue;
        }
        if overwear_set.contains(&overwear.id) {
            bail!("Multiple definition for overwear {:?}", overwear.id);
        }
        overwear_set.insert(overwear.id);
        let series = series_map.get_mut(&overwear.series).with_context(|| {
            format!(
                "Cannot find series {:?} for overwear {:?}",
                overwear.series, overwear.id
            )
        })?;
        let slot = match overwear.id {
            PlOverwearId::Head(_) => 0,
            PlOverwearId::Chest(_) => 1,
            PlOverwearId::Arm(_) => 2,
            PlOverwearId::Waist(_) => 3,
            PlOverwearId::Leg(_) => 4,
        };
        let armor = series.pieces[slot]
            .as_mut()
            .with_context(|| format!("No armor slot for for overwear {:?}", overwear.id))?;
        if armor.data.pl_armor_id != overwear.relative_id {
            bail!("Mismatch armor ID for overwear {:?}", overwear.id);
        }
        if armor.overwear.is_some() {
            bail!("Multiple definition for overwear {:?}", overwear.id);
        }
        armor.overwear = Some(overwear);
        armor.overwear_product = overwear_product_map.remove(&overwear.id);
    }

    Ok(series_map.into_iter().map(|(_, v)| v).collect())
}

fn prepare_meat_names(pedia: &Pedia) -> Result<HashMap<MeatKey, &MsgEntry>> {
    let msg_map: HashMap<_, _> = pedia.hunter_note_msg.get_name_map();

    let mut result = HashMap::new();

    for boss_monster in &pedia.monster_list.data_list {
        for part_data in &boss_monster.part_table_data {
            let part = part_data.em_meat.try_into()?;
            let phase = part_data.em_meat_group_index.try_into()?;
            let key = MeatKey {
                em_type: boss_monster.em_type,
                part,
                phase,
            };

            let name = if let Some(&name) = msg_map.get(&format!(
                "HN_Hunternote_ML_Tab_02_Parts{:02}",
                part_data.part
            )) {
                name
            } else {
                continue;
            };

            if result.insert(key, name).is_some() {
                bail!(
                    "Duplicate definition for meat {:?}-{}-{}",
                    boss_monster.em_type,
                    part,
                    phase
                );
            }
        }
    }

    Ok(result)
}

fn prepare_items<'a>(pedia: &'a Pedia) -> Result<BTreeMap<ItemId, Item<'a>>> {
    let mut result: BTreeMap<ItemId, Item<'a>> = BTreeMap::new();
    let mut name_map: HashMap<_, _> = pedia.items_name_msg.get_name_map();
    let mut explain_map: HashMap<_, _> = pedia.items_explain_msg.get_name_map();
    for param in &pedia.items.param {
        if let Some(existing) = result.get_mut(&param.id) {
            eprintln!("Duplicate definition for item {:?}", param.id);
            existing.multiple_def = true;
            continue;
        }

        let (name_tag, explain_tag) = match param.id {
            ItemId::Normal(id) => (
                format!("I_{:04}_Name", id & 0xFFFF),
                format!("I_{:04}_Explain", id & 0xFFFF),
            ),
            _ => bail!("Unexpected item type"),
        };

        let name = name_map
            .remove(&name_tag)
            .with_context(|| format!("Name not found for item {:?}", param.id))?;

        let explain = explain_map
            .remove(&explain_tag)
            .with_context(|| format!("Explain not found for item {:?}", param.id))?;
        let item = Item {
            name,
            explain,
            param,
            multiple_def: false,
        };
        result.insert(param.id, item);
    }

    Ok(result)
}

static AMMOR_SPHERE_CATEGORY_MSG: Lazy<MsgEntry> = Lazy::new(|| MsgEntry {
    name: "".to_string(),
    guid: Guid { bytes: [0; 16] },
    hash: 0,
    attributes: vec![],
    content: vec!["Armor sphere".to_string(); 32],
});

fn prepare_material_categories(pedia: &Pedia) -> HashMap<MaterialCategory, &MsgEntry> {
    const PREFIX: &str = "ICT_Name_";
    pedia
        .material_category_msg
        .entries
        .iter()
        .filter_map(|entry| {
            if !entry.name.starts_with(PREFIX) {
                return None;
            }

            Some((
                MaterialCategory(entry.name[PREFIX.len()..].parse().ok()?),
                entry,
            ))
        })
        .chain(std::iter::once((
            MaterialCategory(86),
            &*AMMOR_SPHERE_CATEGORY_MSG,
        )))
        .collect()
}

fn prepare_weapon<T, Param>(weapon_list: &WeaponList<T>) -> Result<WeaponTree<'_, Param>>
where
    T: Deref<Target = [Param]>,
    Param: ToBase<WeaponBaseData>,
{
    let mut product_map = HashMap::new();
    for product in &weapon_list.product.param {
        if product.base.id == WeaponId::None || product.base.id == WeaponId::Null {
            continue;
        }
        if product_map.insert(product.base.id, product).is_some() {
            bail!("Multiple product for weapon {:?}", product.base.id);
        }
    }

    let mut change_map = HashMap::new();
    for change in &weapon_list.change.param {
        if change.base.id == WeaponId::None || change.base.id == WeaponId::Null {
            continue;
        }
        if change_map.insert(change.base.id, change).is_some() {
            bail!("Multiple change for weapon {:?}", change.base.id);
        }
    }

    let mut process_map = HashMap::new();
    for process in &weapon_list.process.param {
        if process.base.id == WeaponId::None || process.base.id == WeaponId::Null {
            continue;
        }
        if process_map.insert(process.base.id, process).is_some() {
            bail!("Multiple process for weapon {:?}", process.base.id);
        }
    }

    let mut name_map = weapon_list.name.get_name_map();
    let mut explain_map = weapon_list.explain.get_name_map();

    let mut weapons = HashMap::new();
    for param in &*weapon_list.base_data {
        let id = param.to_base().id;
        if id == WeaponId::None || id == WeaponId::Null {
            continue;
        }
        if weapons.contains_key(&id) {
            bail!("Multiple definition for weapon {:?}", param.to_base().id)
        }
        let tag = id.to_tag();
        let name = name_map.remove(&format!("W_{}_Name", tag)).unwrap();
        let explain = explain_map.remove(&format!("W_{}_Explain", tag)).unwrap();

        let weapon = Weapon {
            param,
            product: product_map.remove(&id),
            change: change_map.remove(&id),
            process: process_map.remove(&id),
            name,
            explain,
            children: vec![],
            parent: None,
        };
        weapons.insert(param.to_base().id, weapon);
    }

    if !product_map.is_empty() {
        bail!("Left over product data: {:?}", product_map)
    }

    if !process_map.is_empty() {
        bail!("Left over process data: {:?}", process_map)
    }

    if !change_map.is_empty() {
        bail!("Left over change data: {:?}", change_map)
    }

    let mut tree_map = HashMap::new();
    let mut tree_id_set = HashSet::new();
    for node in &weapon_list.tree.param {
        if node.weapon_id == WeaponId::None || node.weapon_id == WeaponId::Null {
            continue;
        }
        if tree_id_set.contains(&node.weapon_id) {
            bail!("Multiple tree node for weapon {:?}", node.weapon_id)
        }
        if !weapons.contains_key(&node.weapon_id) {
            bail!("Unknown weapon in tree node {:?}", node.weapon_id);
        }
        tree_id_set.insert(node.weapon_id);
        if tree_map
            .insert((node.tree_type, node.index), node)
            .is_some()
        {
            bail!(
                "Multiple weapon at position {:?}",
                (node.tree_type, node.index)
            )
        }
    }

    let mut unpositioned = vec![];

    for weapon in weapons.keys() {
        if !tree_id_set.contains(weapon) {
            unpositioned.push(*weapon);
        }
    }

    let mut roots = vec![];

    for node in &weapon_list.tree.param {
        if node.weapon_id == WeaponId::None || node.weapon_id == WeaponId::Null {
            continue;
        }
        if node.prev_weapon_type == TreeType::None {
            roots.push(node.weapon_id);
        } else {
            let prev = tree_map
                .get(&(node.prev_weapon_type, node.prev_weapon_index))
                .with_context(|| format!("Unknown previous position for {:?}", node))?;
            weapons.get_mut(&node.weapon_id).unwrap().parent = Some(prev.weapon_id);
            if !prev
                .next_weapon_type_list
                .iter()
                .cloned()
                .zip(prev.next_weapon_index_list.iter().cloned())
                .any(|ti| ti == (node.tree_type, node.index))
            {
                bail!("Previous node doesn't contain next for {:?}", node)
            }
        }
        let mut children: Vec<_> = node
            .next_weapon_type_list
            .iter()
            .cloned()
            .zip(node.next_weapon_index_list.iter().cloned())
            .filter(|&(t, _)| t != TreeType::None)
            .collect();
        children.sort_by_key(|&(t, i)| {
            if t == node.tree_type {
                (TreeType::None, -1)
            } else {
                (t, i)
            }
        });
        for (t, i) in children {
            let next = if let Some(next) = tree_map.get(&(t, i)) {
                next
            } else {
                eprintln!("Unknown children at {:?}, {}", t, i);
                continue;
            };
            if next.prev_weapon_type != node.tree_type || next.prev_weapon_index != node.index {
                bail!("Mismatch prev type/index")
            }
            weapons
                .get_mut(&node.weapon_id)
                .unwrap()
                .children
                .push(next.weapon_id);
        }
    }

    let result = WeaponTree {
        weapons,
        unpositioned,
        roots,
    };

    Ok(result)
}

fn prepare_monster_lot(
    pedia: &Pedia,
) -> Result<HashMap<(EmTypes, QuestRank), &MonsterLotTableUserDataParam>> {
    let mut result = HashMap::new();

    for lot in &pedia.monster_lot.param {
        if lot.em_types == EmTypes::Em(0) {
            continue;
        }

        if result.insert((lot.em_types, lot.quest_rank), lot).is_some() {
            bail!(
                "Multiple LOT definition for {:?} {:?}",
                lot.em_types,
                lot.quest_rank
            );
        }
    }

    Ok(result)
}

fn prepare_parts_dictionary(
    pedia: &Pedia,
) -> Result<HashMap<(EmTypes, BrokenPartsTypes), &MsgEntry>> {
    let msgs: HashMap<_, _> = pedia.hunter_note_msg.get_guid_map();

    let mut result = HashMap::new();

    for part in &pedia.parts_type.params {
        for info in &part.text_infos {
            let msg = *msgs.get(&info.text_for_monster_list).with_context(|| {
                format!("Cannot found part text for {:?}", part.broken_parts_types)
            })?;
            for &em in &info.enemy_type_list {
                if em == EmTypes::Em(0) {
                    continue;
                }
                if result.insert((em, part.broken_parts_types), msg).is_some() {
                    eprintln!(
                        "Multiple part text for {:?}, {:?}",
                        em, part.broken_parts_types
                    );
                }
            }
        }
    }

    Ok(result)
}

fn prepare_horn_melody(pedia: &Pedia) -> HashMap<i32, &'_ MsgEntry> {
    let mut res = HashMap::new();
    let map = pedia.horn_melody.get_name_map();
    for id in 0..999 {
        let name = format!("Horn_UniqueParam_{:03}_Name", id);
        if let Some(&name) = map.get(&name) {
            res.insert(id, name);
        }
    }
    res
}

pub fn gen_pedia_ex(pedia: &Pedia) -> Result<PediaEx<'_>> {
    Ok(PediaEx {
        sizes: prepare_size_map(&pedia.size_list)?,
        size_dists: prepare_size_dist_map(&pedia.random_scale)?,
        quests: prepare_quests(pedia)?,
        discoveries: prepare_discoveries(pedia)?,
        skills: prepare_skills(pedia)?,
        hyakuryu_skills: prepare_hyakuryu_skills(pedia)?,
        armors: prepare_armors(pedia)?,
        meat_names: prepare_meat_names(pedia)?,
        items: prepare_items(pedia)?,
        material_categories: prepare_material_categories(pedia),
        monster_lot: prepare_monster_lot(pedia)?,
        parts_dictionary: prepare_parts_dictionary(pedia)?,

        great_sword: prepare_weapon(&pedia.great_sword)?,
        short_sword: prepare_weapon(&pedia.short_sword)?,
        hammer: prepare_weapon(&pedia.hammer)?,
        lance: prepare_weapon(&pedia.lance)?,
        long_sword: prepare_weapon(&pedia.long_sword)?,
        slash_axe: prepare_weapon(&pedia.slash_axe)?,
        gun_lance: prepare_weapon(&pedia.gun_lance)?,
        dual_blades: prepare_weapon(&pedia.dual_blades)?,
        horn: prepare_weapon(&pedia.horn)?,
        insect_glaive: prepare_weapon(&pedia.insect_glaive)?,
        charge_axe: prepare_weapon(&pedia.charge_axe)?,
        light_bowgun: prepare_weapon(&pedia.light_bowgun)?,
        heavy_bowgun: prepare_weapon(&pedia.heavy_bowgun)?,
        bow: prepare_weapon(&pedia.bow)?,
        horn_melody: prepare_horn_melody(pedia),
    })
}

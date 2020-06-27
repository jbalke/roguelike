use super::{
    AreaOfEffect, CombatStats, Confusion, Consumable, GameLog, InBackpack, InflictsDamage, Map,
    Name, Position, ProvidesHealing, SufferDamage, WantsToDropItem, WantsToPickupItem,
    WantsToUseItem,
};
use specs::prelude::*;

pub struct ItemCollectionSystem {}

impl<'a> System<'a> for ItemCollectionSystem {
    type SystemData = (
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        WriteStorage<'a, WantsToPickupItem>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (player_entity, mut gamelog, mut wants_pickup, mut positions, names, mut backpack) =
            data;

        for pickup in wants_pickup.join() {
            positions.remove(pickup.item);
            backpack
                .insert(
                    pickup.item,
                    InBackpack {
                        owner: pickup.collected_by,
                    },
                )
                .expect("Unable to insert backpack entry");

            if pickup.collected_by == *player_entity {
                gamelog.entries.push(format!(
                    "You pick up the {}.",
                    names.get(pickup.item).unwrap().name
                ));
            }
        }

        wants_pickup.clear();
    }
}

pub struct ItemUseSystem {}

impl<'a> System<'a> for ItemUseSystem {
    type SystemData = (
        ReadExpect<'a, Map>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToUseItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Consumable>,
        ReadStorage<'a, ProvidesHealing>,
        WriteStorage<'a, CombatStats>,
        ReadStorage<'a, InflictsDamage>,
        WriteStorage<'a, SufferDamage>,
        ReadStorage<'a, AreaOfEffect>,
        WriteStorage<'a, Confusion>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            player_entity,
            mut gamelog,
            entities,
            mut wants_use,
            names,
            mut consumables,
            healing,
            mut combat_stats,
            inflict_damage,
            mut suffer_damage,
            aoe,
            mut confused,
        ) = data;

        for (entity, useitem) in (&entities, &wants_use).join() {
            let mut used_item = true;

            // Targeting
            let mut targets = Vec::new();
            match useitem.target {
                None => {
                    targets.push(*player_entity);
                }
                Some(target) => {
                    let area_of_effect = aoe.get(useitem.item);
                    match area_of_effect {
                        Some(aoe) => {
                            // AoE
                            let mut blast_tiles = rltk::field_of_view(target, aoe.radius, &*map);
                            blast_tiles.retain(|p| {
                                p.x > 0 && p.x < map.width - 1 && p.y > 0 && p.y < map.height - 1
                            });

                            for tile_idx in blast_tiles.iter() {
                                let idx = map.xy_idx(tile_idx.x, tile_idx.y);
                                for mob in map.tile_content[idx].iter() {
                                    targets.push(*mob);
                                }
                            }
                        }
                        None => {
                            // Single target in tile
                            let idx = map.xy_idx(target.x, target.y);
                            for mob in map.tile_content[idx].iter() {
                                targets.push(*mob);
                            }
                        }
                    }
                }
            }

            // If a healing item, apply the healing.
            if let Some(healer) = healing.get(useitem.item) {
                used_item = false;

                for target in targets.iter() {
                    if let Some(stats) = combat_stats.get_mut(*target) {
                        stats.hp = i32::min(stats.max_hp, stats.hp + healer.heal_amount);
                        if entity == *player_entity {
                            gamelog.entries.push(format!(
                                "You use the {}, healing {} hp.",
                                names.get(useitem.item).unwrap().name,
                                healer.heal_amount
                            ));
                        }
                        used_item = true;
                    }
                }
            }

            let mut add_confusion = Vec::new();
            if let Some(confusion) = confused.get(useitem.item) {
                used_item = false;

                for mob in targets.iter() {
                    add_confusion.push((*mob, confusion.turns));

                    if entity == *player_entity {
                        let mob_name = &names.get(*mob).unwrap().name;
                        let item_name = &names.get(useitem.item).unwrap().name;
                        gamelog.entries.push(format!(
                            "You use {} on {}, confusing them.",
                            item_name, mob_name,
                        ));
                    }
                }

                for (mob, turns) in add_confusion.iter() {
                    confused
                        .insert(*mob, Confusion { turns: *turns })
                        .expect("Unable to insert status");

                    used_item = true;
                }
            }

            // If it inflicts damage, apply it to the target cell
            if let Some(damage) = inflict_damage.get(useitem.item) {
                used_item = false;
                for mob in targets.iter() {
                    SufferDamage::new_damage(&mut suffer_damage, *mob, damage.damage);
                    if entity == *player_entity {
                        let mob_name = names.get(*mob).unwrap();
                        let item_name = names.get(useitem.item).unwrap();
                        gamelog.entries.push(format!(
                            "You use {} on {}, inflicting {} hp.",
                            item_name.name, mob_name.name, damage.damage
                        ));
                    }

                    used_item = true;
                }
            }

            if used_item {
                if let Some(consumable) = consumables.get_mut(useitem.item) {
                    if consumable.uses <= 1 {
                        entities.delete(useitem.item).expect("Delete failed");
                    } else {
                        consumable.uses -= 1;
                    }
                }
            }
        }

        wants_use.clear();
    }
}

pub struct ItemDropSystem {}

impl<'a> System<'a> for ItemDropSystem {
    type SystemData = (
        ReadExpect<'a, Map>,
        ReadExpect<'a, Entity>,
        WriteExpect<'a, GameLog>,
        Entities<'a>,
        WriteStorage<'a, WantsToDropItem>,
        ReadStorage<'a, Name>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, InBackpack>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            map,
            player_entity,
            mut gamelog,
            entities,
            mut wants_drop,
            names,
            mut positions,
            mut backpack,
        ) = data;

        for (entity, to_drop) in (&entities, &wants_drop).join() {
            let mut dropper_pos: Position = Position { x: 0, y: 0 };
            let dropped_pos = positions.get(entity).unwrap();
            dropper_pos.x = dropped_pos.x;
            dropper_pos.y = dropped_pos.y;

            let idx = map.xy_idx(dropper_pos.x, dropper_pos.y);
            let item_name = &names.get(to_drop.item).unwrap().name;
            if map.tile_content[idx].len() > 1 {
                if entity == *player_entity {
                    gamelog
                        .entries
                        .push(format!("You can not drop {} here.", item_name));
                }
            } else {
                positions
                    .insert(
                        to_drop.item,
                        Position {
                            x: dropper_pos.x,
                            y: dropper_pos.y,
                        },
                    )
                    .expect("Unable to insert position");
                backpack.remove(to_drop.item);

                if entity == *player_entity {
                    gamelog.entries.push(format!("You drop the {}.", item_name));
                }
            }
        }

        wants_drop.clear();
    }
}

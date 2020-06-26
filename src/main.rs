use rltk::{GameState, Point, Rltk};
use specs::prelude::*;

mod gamelog;
mod gui;
mod spawner;
pub use gamelog::GameLog;
mod components;
pub use components::*;
mod map;
pub use map::*;
mod player;
use player::*;
mod rect;
pub use rect::Rect;
mod visibility_system;
use visibility_system::VisibilitySystem;
mod monster_ai_system;
use monster_ai_system::MonsterAI;
mod map_indexing_system;
use map_indexing_system::MapIndexingSystem;
mod melee_combat_system;
use melee_combat_system::MeleeCombatSystem;
mod damage_system;
use damage_system::DamageSystem;
mod inventory_system;
use inventory_system::{ItemCollectionSystem, ItemDropSystem, ItemUseSystem};

#[derive(PartialEq, Copy, Clone)]
pub enum RunState {
    AwaitingInput,
    PreRun,
    PlayerTurn,
    MonsterTurn,
    ShowInventory,
    ShowDropItem,
    ShowTargeting { range: i32, item: Entity },
}

pub struct State {
    pub ecs: World,
}

impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem {};
        vis.run_now(&self.ecs);
        let mut mob = MonsterAI {};
        mob.run_now(&self.ecs);
        let mut mapindex = MapIndexingSystem {};
        mapindex.run_now(&self.ecs);
        let mut melee = MeleeCombatSystem {};
        melee.run_now(&self.ecs);
        let mut damage = DamageSystem {};
        damage.run_now(&self.ecs);
        let mut pickup = ItemCollectionSystem {};
        pickup.run_now(&self.ecs);
        let mut use_items = ItemUseSystem {};
        use_items.run_now(&self.ecs);
        let mut drop_items = ItemDropSystem {};
        drop_items.run_now(&self.ecs);
        self.ecs.maintain();
    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut Rltk) {
        ctx.cls();

        draw_map(&self.ecs, ctx);
        {
            let positions = self.ecs.read_storage::<Position>();
            let renderables = self.ecs.read_storage::<Renderable>();
            let map = self.ecs.fetch::<Map>();

            // sort renderables by render_order before drawing
            let mut data = (&positions, &renderables).join().collect::<Vec<_>>();
            data.sort_by(|a, b| b.1.render_order.cmp(&a.1.render_order));
            for (pos, render) in data.iter() {
                let idx = map.xy_idx(pos.x, pos.y);
                if map.visible_tiles[idx] {
                    ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph)
                }
            }

            gui::draw_ui(&self.ecs, ctx)
        }

        let mut newrunstate;
        {
            let runstate = self.ecs.fetch::<RunState>();
            newrunstate = *runstate;
        }

        match newrunstate {
            RunState::PreRun => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::AwaitingInput => {
                newrunstate = player_input(self, ctx);
            }
            RunState::PlayerTurn => {
                self.run_systems();
                newrunstate = RunState::MonsterTurn;
            }
            RunState::MonsterTurn => {
                self.run_systems();
                newrunstate = RunState::AwaitingInput;
            }
            RunState::ShowInventory => {
                let (response, item) = gui::show_inventory(self, ctx);
                match response {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = item.unwrap();
                        let is_ranged = self.ecs.read_storage::<Ranged>();
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();

                        if let Some(ranged_item) = is_ranged.get(item_entity) {
                            newrunstate = RunState::ShowTargeting {
                                range: ranged_item.range,
                                item: item_entity,
                            };
                        } else {
                            intent
                                .insert(
                                    *self.ecs.fetch::<Entity>(),
                                    WantsToUseItem {
                                        item: item_entity,
                                        target: None,
                                    },
                                )
                                .expect("Unable to insert intent");
                            newrunstate = RunState::PlayerTurn;
                        }
                    }
                }
            }
            RunState::ShowDropItem => {
                let (response, item) = gui::drop_item_menu(self, ctx);
                match response {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let item_entity = item.unwrap();
                        let mut intent = self.ecs.write_storage::<WantsToDropItem>();
                        intent
                            .insert(
                                *self.ecs.fetch::<Entity>(),
                                WantsToDropItem { item: item_entity },
                            )
                            .expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
            RunState::ShowTargeting { range, item } => {
                let (response, target) = gui::ranged_target(self, ctx, range);
                match response {
                    gui::ItemMenuResult::Cancel => newrunstate = RunState::AwaitingInput,
                    gui::ItemMenuResult::NoResponse => {}
                    gui::ItemMenuResult::Selected => {
                        let mut intent = self.ecs.write_storage::<WantsToUseItem>();
                        intent
                            .insert(*self.ecs.fetch::<Entity>(), WantsToUseItem { item, target })
                            .expect("Unable to insert intent");
                        newrunstate = RunState::PlayerTurn;
                    }
                }
            }
        }

        {
            let mut runwriter = self.ecs.write_resource::<RunState>();
            *runwriter = newrunstate;
        }
        damage_system::delete_the_dead(&mut self.ecs);
    }
}

fn main() -> rltk::BError {
    use rltk::RltkBuilder;
    let mut context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;
    context.with_post_scanlines(true);
    let mut gs = State { ecs: World::new() };
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<Player>();
    gs.ecs.register::<Viewshed>();
    gs.ecs.register::<Monster>();
    gs.ecs.register::<Name>();
    gs.ecs.register::<BlocksTile>();
    gs.ecs.register::<CombatStats>();
    gs.ecs.register::<WantsToMelee>();
    gs.ecs.register::<SufferDamage>();
    gs.ecs.register::<Item>();
    gs.ecs.register::<Potion>();
    gs.ecs.register::<InBackpack>();
    gs.ecs.register::<WantsToPickupItem>();
    gs.ecs.register::<WantsToUseItem>();
    gs.ecs.register::<WantsToDropItem>();
    gs.ecs.register::<Consumable>();
    gs.ecs.register::<ProvidesHealing>();
    gs.ecs.register::<Ranged>();
    gs.ecs.register::<InflictsDamage>();
    gs.ecs.register::<AreaOfEffect>();

    let map: Map = map::new_map_rooms_and_corridors();
    let (player_x, player_y) = map.rooms[0].center();

    let player_entity = spawner::player(&mut gs.ecs, player_x, player_y);

    gs.ecs.insert(rltk::RandomNumberGenerator::new());
    for room in map.rooms.iter().skip(1) {
        spawner::spawn_room(&mut gs.ecs, room);
    }

    gs.ecs.insert(map);
    gs.ecs.insert(Point::new(player_x, player_y));
    gs.ecs.insert(player_entity);
    gs.ecs.insert(RunState::PreRun);
    gs.ecs.insert(GameLog {
        entries: vec!["Welcome to the Rusty RogueLike".to_string()],
    });

    rltk::main_loop(context, gs)
}

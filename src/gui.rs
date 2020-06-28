use super::{
    CombatStats, Consumable, GameLog, InBackpack, Map, Name, Player, Position, RunState, State,
    Viewshed, MAPHEIGHT, MAPWIDTH,
};
use rltk::{Point, Rltk, VirtualKeyCode, RGB};
use specs::prelude::*;

const LOG_LINES: usize = 5;

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuSelection {
    NewGame,
    LoadGame,
    Quit,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MainMenuResult {
    NoSelection { selected: MainMenuSelection },
    Selected { selected: MainMenuSelection },
}

#[derive(PartialEq, Copy, Clone)]
pub enum ItemMenuResult {
    Cancel,
    NoResponse,
    Selected,
}

pub fn main_menu(gs: &mut State, ctx: &mut Rltk) -> MainMenuResult {
    let runstate = gs.ecs.fetch::<RunState>();

    ctx.print_color_centered(
        15,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Rusty Roguelike",
    );

    if let RunState::MainMenu {
        menu_selection: selection,
    } = *runstate
    {
        if selection == MainMenuSelection::NewGame {
            ctx.print_color_centered(
                24,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Begin New Game",
            );
        } else {
            ctx.print_color_centered(
                24,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::BLACK),
                "Begin New Game",
            );
        }
        if selection == MainMenuSelection::LoadGame {
            ctx.print_color_centered(
                25,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Load Game",
            );
        } else {
            ctx.print_color_centered(
                25,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::BLACK),
                "Load Game",
            );
        }
        if selection == MainMenuSelection::Quit {
            ctx.print_color_centered(
                26,
                RGB::named(rltk::MAGENTA),
                RGB::named(rltk::BLACK),
                "Quit Game",
            );
        } else {
            ctx.print_color_centered(
                26,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::BLACK),
                "Quit Game",
            );
        }

        match ctx.key {
            None => {
                return MainMenuResult::NoSelection {
                    selected: selection,
                }
            }
            Some(key) => match key {
                VirtualKeyCode::Escape => {
                    return MainMenuResult::NoSelection {
                        selected: MainMenuSelection::Quit,
                    }
                }
                VirtualKeyCode::Up => {
                    let new_selection;
                    match selection {
                        MainMenuSelection::NewGame => new_selection = MainMenuSelection::Quit,
                        MainMenuSelection::LoadGame => new_selection = MainMenuSelection::NewGame,
                        MainMenuSelection::Quit => new_selection = MainMenuSelection::LoadGame,
                    }
                    return MainMenuResult::NoSelection {
                        selected: new_selection,
                    };
                }
                VirtualKeyCode::Down => {
                    let new_selection;
                    match selection {
                        MainMenuSelection::NewGame => new_selection = MainMenuSelection::LoadGame,
                        MainMenuSelection::LoadGame => new_selection = MainMenuSelection::Quit,
                        MainMenuSelection::Quit => new_selection = MainMenuSelection::NewGame,
                    }
                    return MainMenuResult::NoSelection {
                        selected: new_selection,
                    };
                }
                VirtualKeyCode::Return => {
                    return MainMenuResult::Selected {
                        selected: selection,
                    }
                }
                _ => {
                    return MainMenuResult::NoSelection {
                        selected: selection,
                    }
                }
            },
        }
    }
    MainMenuResult::NoSelection {
        selected: MainMenuSelection::NewGame,
    }
}

pub fn ranged_target(
    gs: &mut State,
    ctx: &mut Rltk,
    range: i32,
    radius: i32,
) -> (ItemMenuResult, Option<Point>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let player_pos = gs.ecs.fetch::<Point>();
    let viewsheds = gs.ecs.read_storage::<Viewshed>();

    ctx.print_color(
        5,
        0,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Select target:",
    );

    // Highlight available targets
    let mut available_cells = Vec::new();
    let player_viewshed = viewsheds.get(*player_entity);

    match player_viewshed {
        Some(visible) => {
            for idx in visible.visible_tiles.iter() {
                let distance = rltk::DistanceAlg::Pythagoras.distance2d(*player_pos, *idx);
                if distance <= range as f32 {
                    ctx.set_bg(idx.x, idx.y, RGB::named(rltk::BLUE));
                    available_cells.push(idx);
                }
            }
        }
        None => return (ItemMenuResult::Cancel, None),
    }

    // Draw mouse cursor
    let mouse_pos = ctx.mouse_pos();
    let mut valid_target = false;

    for idx in available_cells.iter() {
        if idx.x == mouse_pos.0 && idx.y == mouse_pos.1 {
            valid_target = true;
        }
    }

    if valid_target {
        if radius > 0 {
            let map = gs.ecs.fetch::<Map>();
            let visible = player_viewshed.unwrap();

            let mut blast_tiles =
                rltk::field_of_view(Point::new(mouse_pos.0, mouse_pos.1), radius, &*map);

            blast_tiles.retain(|p| visible.visible_tiles.contains(p));

            for tile in blast_tiles.iter() {
                ctx.set_bg(tile.x, tile.y, RGB::named(rltk::ORANGE));
            }
        }

        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::CYAN));
        if ctx.left_click {
            return (
                ItemMenuResult::Selected,
                Some(Point::new(mouse_pos.0, mouse_pos.1)),
            );
        }
    } else {
        ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::RED));
        if ctx.left_click {
            return (ItemMenuResult::Cancel, None);
        }
    }
    (ItemMenuResult::NoResponse, None)
}

pub fn drop_item_menu(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();

    let inventory = (&backpack, &names)
        .join()
        .filter(|(item, _)| item.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        31,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Drop Which Item?",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESCAPE to cancel",
    );

    let mut equippable: Vec<Entity> = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names)
        .join()
        .filter(|(_, item, _)| item.owner == *player_entity)
    {
        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );

        ctx.print(21, y, &name.name.to_string());
        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if !selection.is_negative() && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

pub fn show_inventory(gs: &mut State, ctx: &mut Rltk) -> (ItemMenuResult, Option<Entity>) {
    let player_entity = gs.ecs.fetch::<Entity>();
    let names = gs.ecs.read_storage::<Name>();
    let backpack = gs.ecs.read_storage::<InBackpack>();
    let entities = gs.ecs.entities();
    let consumables = gs.ecs.read_storage::<Consumable>();

    let inventory = (&backpack, &names)
        .join()
        .filter(|(item, _)| item.owner == *player_entity);
    let count = inventory.count();

    let mut y = (25 - (count / 2)) as i32;
    ctx.draw_box(
        15,
        y - 2,
        34,
        (count + 3) as i32,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );
    ctx.print_color(
        18,
        y - 2,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "Inventory",
    );
    ctx.print_color(
        18,
        y + count as i32 + 1,
        RGB::named(rltk::YELLOW),
        RGB::named(rltk::BLACK),
        "ESC to cancel",
    );

    let mut equippable = Vec::new();
    let mut j = 0;
    for (entity, _pack, name) in (&entities, &backpack, &names)
        .join()
        .filter(|(_, item, _)| item.owner == *player_entity)
    {
        let mut uses = 1;
        if let Some(consumable) = consumables.get(entity) {
            uses = consumable.uses;
        }

        ctx.set(
            17,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437('('),
        );
        ctx.set(
            18,
            y,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            97 + j as rltk::FontCharType,
        );
        ctx.set(
            19,
            y,
            RGB::named(rltk::WHITE),
            RGB::named(rltk::BLACK),
            rltk::to_cp437(')'),
        );
        let mut item_decription = name.name.to_string();
        if uses > 1 {
            item_decription.push_str(&format!(" ({})", uses))
        }
        ctx.print(21, y, &item_decription);

        equippable.push(entity);
        y += 1;
        j += 1;
    }

    match ctx.key {
        None => (ItemMenuResult::NoResponse, None),
        Some(key) => match key {
            VirtualKeyCode::Escape => (ItemMenuResult::Cancel, None),
            _ => {
                let selection = rltk::letter_to_option(key);
                if !selection.is_negative() && selection < count as i32 {
                    return (
                        ItemMenuResult::Selected,
                        Some(equippable[selection as usize]),
                    );
                }
                (ItemMenuResult::NoResponse, None)
            }
        },
    }
}

pub fn draw_ui(ecs: &World, ctx: &mut Rltk) {
    ctx.draw_box(
        0,
        MAPHEIGHT,
        MAPWIDTH - 1,
        6,
        RGB::named(rltk::WHITE),
        RGB::named(rltk::BLACK),
    );

    let combat_stats = ecs.read_storage::<CombatStats>();
    let players = ecs.read_storage::<Player>();
    for (_player, stats) in (&players, &combat_stats).join() {
        let health = format!(" HP: {} / {} ", stats.hp, stats.max_hp);
        ctx.print_color(
            12,
            MAPHEIGHT,
            RGB::named(rltk::YELLOW),
            RGB::named(rltk::BLACK),
            &health,
        );

        ctx.draw_bar_horizontal(
            28,
            MAPHEIGHT,
            51,
            stats.hp,
            stats.max_hp,
            RGB::named(rltk::RED),
            RGB::named(rltk::BLACK),
        );
    }

    let log = ecs.fetch::<GameLog>();
    let mut y = MAPHEIGHT + 1;

    for s in log.entries.iter().rev().take(LOG_LINES) {
        ctx.print(2, y, s);
        y += 1;
    }

    let mouse_pos = ctx.mouse_pos();
    ctx.set_bg(mouse_pos.0, mouse_pos.1, RGB::named(rltk::MAGENTA));

    draw_tooltips(ecs, ctx);
}

fn draw_tooltips(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let names = ecs.read_storage::<Name>();
    let positions = ecs.read_storage::<Position>();
    let mouse_pos = ctx.mouse_pos();

    if mouse_pos.0 >= map.width || mouse_pos.1 >= map.height {
        return;
    }

    let mut tooltip = Vec::new();
    for (name, position) in (&names, &positions).join() {
        let idx = map.xy_idx(position.x, position.y);
        if position.x == mouse_pos.0 && position.y == mouse_pos.1 && map.visible_tiles[idx] {
            tooltip.push(name.name.to_string());
        }
    }

    if !tooltip.is_empty() {
        let width = tooltip.iter().map(|s| s.len()).max().unwrap() as i32 + 3;

        if mouse_pos.0 > 40 {
            let arrow_pos = Point::new(mouse_pos.0 - 2, mouse_pos.1);
            let left_x = mouse_pos.0 - width;
            let mut y = mouse_pos.1;

            for s in tooltip.iter() {
                ctx.print_color(
                    left_x,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GREY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x - i,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GREY),
                        " ",
                    );
                }
                y += 1;
            }
            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GREY),
                "->",
            );
        } else {
            let arrow_pos = Point::new(mouse_pos.0 + 1, mouse_pos.1);
            let left_x = mouse_pos.0 + 3;
            let mut y = mouse_pos.1;
            for s in tooltip.iter() {
                ctx.print_color(
                    left_x + 1,
                    y,
                    RGB::named(rltk::WHITE),
                    RGB::named(rltk::GREY),
                    s,
                );
                let padding = (width - s.len() as i32) - 1;
                for i in 0..padding {
                    ctx.print_color(
                        arrow_pos.x + 1 + i,
                        y,
                        RGB::named(rltk::WHITE),
                        RGB::named(rltk::GREY),
                        " ",
                    );
                }
                y += 1;
            }
            ctx.print_color(
                arrow_pos.x,
                arrow_pos.y,
                RGB::named(rltk::WHITE),
                RGB::named(rltk::GREY),
                "<-",
            );
        }
    }
}

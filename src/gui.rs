use super::{CombatStats, Player, MAPHEIGHT, MAPWIDTH};
use rltk::{Console, Rltk, RGB};
use specs::prelude::*;

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
}

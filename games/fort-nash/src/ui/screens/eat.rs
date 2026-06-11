//! Choose how well to eat this week. Eating poorly saves food but courts
//! illness; eating well is safe but hungry on supplies.

use dioxus::prelude::*;

use crate::engine::state::EatLevel;
use crate::engine::Game;
use retro_kit::components::menu_button::MenuButton;
use retro_kit::format::fmt_num;
use retro_kit::theme::SCREEN_CENTERED;

const CHOICES: [(EatLevel, &str); 3] = [
    (EatLevel::Poorly, "barely enough — risky, but it stretches the larder"),
    (EatLevel::Moderately, "a fair ration"),
    (EatLevel::Well, "eat hearty — keeps the party healthy"),
];

#[component]
pub fn Eat() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let food = game.read().state.food.max(0.0);

    rsx! {
        div { class: "{SCREEN_CENTERED} gap-3",
            h2 { class: "text-center text-lg", "How would you like to eat?" }
            p { class: "text-center opacity-70 text-sm", "Food on hand: {fmt_num(food)} lbs" }
            for (level, hint) in CHOICES {
                MenuButton {
                    key: "{level.label()}",
                    title: format!("{} ({} lbs)", level.label(), level.food_cost() as i64),
                    hint: hint.to_string(),
                    disabled: food < level.food_cost(),
                    onclick: move |_| game.write().choose_eat(level),
                }
            }
        }
    }
}

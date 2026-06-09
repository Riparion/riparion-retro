//! Set the year's fiscal policy — three taxes and the severity of your justice.
//! The projected revenue updates live; high taxes raise gold but choke the
//! growth of merchants and nobles, and harsh justice costs you your good name.

use dioxus::prelude::*;

use crate::engine::Game;
use retro_kit::components::stat_row::StatRow;
use retro_kit::format::group_thousands;
use retro_kit::theme::{BTN, BTN_PRIMARY, PANEL, SCREEN};

#[component]
pub fn Tax() -> Element {
    let mut game = use_context::<Signal<Game>>();
    let g = game.read();
    let s = &g.state;
    let customs = s.customs_duty;
    let sales = s.sales_tax;
    let income = s.income_tax;
    let justice = s.justice;
    let justice_label = s.justice_label();
    let (rev_c, rev_s, rev_i, rev_j, total) = g.projected_revenue();
    drop(g);

    rsx! {
        div { class: "{SCREEN} gap-3",
            h2 { class: "text-center text-lg tracking-widest", "FISCAL POLICY" }

            PolicyRow {
                label: "Customs duty",
                value: format!("{customs}%"),
                on_dec: move |_| game.write().set_customs_duty(customs - 5),
                on_inc: move |_| game.write().set_customs_duty(customs + 5),
            }
            PolicyRow {
                label: "Sales tax",
                value: format!("{sales}%"),
                on_dec: move |_| game.write().set_sales_tax(sales - 5),
                on_inc: move |_| game.write().set_sales_tax(sales + 5),
            }
            PolicyRow {
                label: "Wealth tax",
                value: format!("{income}%"),
                on_dec: move |_| game.write().set_income_tax(income - 5),
                on_inc: move |_| game.write().set_income_tax(income + 5),
            }
            PolicyRow {
                label: "Justice",
                value: format!("{justice} · {justice_label}"),
                on_dec: move |_| game.write().set_justice(justice - 1),
                on_inc: move |_| game.write().set_justice(justice + 1),
            }

            div { class: "{PANEL} p-3 flex flex-col gap-1 text-sm",
                p { class: "chip-label mb-1 text-center", "── PROJECTED REVENUE ──" }
                StatRow { label: "Customs duty", value: format!("{} fl", group_thousands(rev_c)) }
                StatRow { label: "Sales tax", value: format!("{} fl", group_thousands(rev_s)) }
                StatRow { label: "Wealth tax", value: format!("{} fl", group_thousands(rev_i)) }
                StatRow { label: "Court of justice", value: format!("{} fl", group_thousands(rev_j)) }
                div { class: "border-t border-current opacity-40 my-1" }
                StatRow { label: "Total this year", value: format!("{} fl", group_thousands(total)) }
            }

            button {
                class: "{BTN_PRIMARY} py-4 text-lg",
                onclick: move |_| game.write().done_tax(),
                "COLLECT TAXES ▸"
            }
        }
    }
}

#[component]
fn PolicyRow(
    label: &'static str,
    value: String,
    on_dec: EventHandler<()>,
    on_inc: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "{PANEL} p-2 flex items-center gap-2",
            div { class: "flex-1",
                div { class: "chip-label", "{label}" }
                div { class: "text-base", "{value}" }
            }
            button { class: "{BTN} px-4 text-xl", onclick: move |_| on_dec.call(()), "−" }
            button { class: "{BTN} px-4 text-xl", onclick: move |_| on_inc.call(()), "+" }
        }
    }
}

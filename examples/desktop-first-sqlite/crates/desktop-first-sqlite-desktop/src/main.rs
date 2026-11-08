use desktop_first_sqlite_core as core;
use rune::prelude::*;

struct DesktopState {
    engine: core::StateEngine,
    status: String,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            engine: core::StateEngine::new(),
            status: "Núcleo Rust conectado via Rune Desktop".to_string(),
        }
    }
}

fn view(s: &DesktopState, _: &mut Cx<DesktopState>) -> El {
    let t = tok::theme(true);
    let core_state = s.engine.get_state();

    let header = col()
        .gap(4.0)
        .child(
            text("🪵 desktop-first-sqlite Desktop")
                .font(type_::TITLE)
                .color(t.text),
        )
        .child(
            text("UI Desktop nativa em GPU via Rune (anomalyco/rune)")
                .font(type_::CAPTION)
                .color(t.text_subtle),
        );

    let counter_val = text(format!("{}", core_state.counter))
        .font(type_::DISPLAY)
        .color(tok::accent(0, true));

    let counter_card = col()
        .w_full()
        .gap(12.0)
        .padding(20.0)
        .radius(12.0)
        .fill(t.card)
        .center()
        .child(text("Contador (Rust Core):").font(type_::LABEL).color(t.text_subtle))
        .child(counter_val)
        .child(text(&s.status).font(type_::CAPTION).color(t.text_subtle));

    let btn_dec = el()
        .role("button")
        .focusable(true)
        .padding_xy(16.0, 10.0)
        .radius(8.0)
        .fill(t.danger)
        .on_press(|s: &mut DesktopState| {
            match s.engine.decrement() {
                Ok(_) => s.status = "Decrementado via Rust StateEngine".to_string(),
                Err(_) => s.status = "Erro: valor não pode ser negativo".to_string(),
            }
        })
        .child(text("- Decrementar").color(t.text));

    let btn_inc = el()
        .role("button")
        .focusable(true)
        .padding_xy(16.0, 10.0)
        .radius(8.0)
        .fill(tok::accent(0, true))
        .on_press(|s: &mut DesktopState| {
            s.engine.increment();
            s.status = "Incrementado via Rust StateEngine".to_string();
        })
        .child(text("+ Incrementar").color(t.text));

    let actions = row().gap(16.0).center().child(btn_dec).child(btn_inc);

    col()
        .w_full()
        .h_full()
        .center()
        .gap(24.0)
        .padding(24.0)
        .fill(t.bg)
        .child(header)
        .child(counter_card)
        .child(actions)
}

fn main() {
    rune::run(AppSpec::new(
        "desktop-first-sqlite Desktop",
        (480.0, 620.0),
        DesktopState::default,
        view,
    ));
}

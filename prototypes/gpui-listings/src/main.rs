use gpui::{
    App, Application, Bounds, Context, Div, Render, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size, uniform_list,
};

const ITEMS: [&str; 6] = [
    "Ash Prime Blueprint",
    "Ember Prime Systems",
    "Frost Prime Chassis",
    "Rhino Prime Neuroptics",
    "Volt Prime Blueprint",
    "Wisp Prime Systems",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Buy,
    Sell,
}

impl Side {
    fn label(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
        }
    }
}

struct Listing {
    item: &'static str,
    side: Side,
    platinum: u32,
    quantity: u32,
}

struct Listings {
    rows: Vec<Listing>,
    visible: Vec<usize>,
    filter: Option<Side>,
}

impl Listings {
    fn sample(count: u32) -> Self {
        let rows = (0..count)
            .map(|index| Listing {
                item: ITEMS[index as usize % ITEMS.len()],
                side: if index % 4 < 2 { Side::Buy } else { Side::Sell },
                platinum: 5 + (index * 7) % 120,
                quantity: 1 + index % 9,
            })
            .collect();
        Self {
            rows,
            visible: (0..count as usize).collect(),
            filter: None,
        }
    }

    fn set_filter(&mut self, filter: Option<Side>) {
        self.filter = filter;
        self.visible = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| filter.is_none_or(|side| side == row.side).then_some(index))
            .collect();
    }
}

fn columns(item: &'static str, side: &'static str, price: String, quantity: String) -> Div {
    div()
        .w_full()
        .flex()
        .items_center()
        .h(px(34.))
        .px_3()
        .child(div().flex_1().truncate().child(item))
        .child(div().w(px(80.)).child(side))
        .child(div().w(px(90.)).child(price))
        .child(div().w(px(80.)).child(quantity))
}

impl Render for Listings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x171a20))
            .text_color(rgb(0xe9edf4))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .child(div().flex_1().child(format!(
                        "Market listings · {} sample orders",
                        self.visible.len()
                    )))
                    .children(
                        [
                            ("All", None),
                            ("Buy", Some(Side::Buy)),
                            ("Sell", Some(Side::Sell)),
                        ]
                        .into_iter()
                        .map(|(label, filter)| {
                            div()
                                .id(label)
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .bg(rgb(if self.filter == filter {
                                    0x365681
                                } else {
                                    0x272c36
                                }))
                                .cursor_pointer()
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.set_filter(filter);
                                    cx.notify();
                                }))
                                .child(label)
                        }),
                    ),
            )
            .child(columns("Item", "Side", "Platinum".into(), "Quantity".into()).bg(rgb(0x272c36)))
            .child(
                uniform_list(
                    match self.filter {
                        None => "all-listings",
                        Some(Side::Buy) => "buy-listings",
                        Some(Side::Sell) => "sell-listings",
                    },
                    self.visible.len(),
                    cx.processor(|view, range: std::ops::Range<usize>, _, _| {
                        range
                            .map(|index| {
                                let row = &view.rows[view.visible[index]];
                                columns(
                                    row.item,
                                    row.side.label(),
                                    row.platinum.to_string(),
                                    row.quantity.to_string(),
                                )
                                .bg(rgb(if index % 2 == 0 {
                                    0x171a20
                                } else {
                                    0x1e222a
                                }))
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1()
                .min_h_0(),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(760.), px(560.)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("QuantFrame listings".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Listings::sample(10_000)),
        );
        if let Err(error) = result {
            eprintln!("Could not open the listings window: {error}");
            std::process::exit(1);
        }
        cx.activate(true);
    });
}

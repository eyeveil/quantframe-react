mod schema;

use std::collections::HashMap;

use gpui::{
    AnyElement, App, Context, Div, Entity, IntoElement, ParentElement, Render, SharedString,
    Stateful, Styled, Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    Disableable, Icon, WindowExt,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
    switch::Switch,
};
use gpui_kit_assets::IconName;
use serde_json::{Value, json};

use crate::{
    data::{Backend, BackendEvent, DataKind, DataSource},
    kit::{self, Loaded},
    model::{Settings, User, UserStatus},
    theme::{AppearanceChoice, Theme, tabular_figures},
    ui::{
        IconButton, Segment, chip, focus_ring,
        form::{self, Field as FormField, FormSpec, FormValues},
        grid::Tone,
        icon, placeholder, segmented, text_button,
    },
};
use schema::{Choices, Extra, Field, Kind, ListSpec, SECTIONS, Section};

const NAV_WIDTH: f32 = 232.;
const CONTENT_WIDTH: f32 = 760.;
const SOURCE_DETAIL: &str = "wf_inventory.source#detail";
const SOURCE_PATH: &str = "wf_inventory.source";

const SOURCES: [(&str, &str, &str, &str); 4] = [
    ("None", "None", "", ""),
    ("Profile", "Warframe profile", "Player id", "id"),
    ("Alecaframe", "AlecaFrame", "Data file", "path"),
    ("File", "Inventory file", "File", "path"),
];

enum Editor {
    Line(Entity<InputState>),
    Area(Entity<TextareaState>),
}

impl Editor {
    fn text(&self, cx: &App) -> String {
        match self {
            Self::Line(input) => input.read(cx).value().to_string(),
            Self::Area(area) => area.read(cx).value().to_string(),
        }
    }

    fn set_text(&self, text: &str, window: &mut Window, cx: &mut App) {
        if self.text(cx) == text {
            return;
        }
        match self {
            Self::Line(input) => {
                input.update(cx, |input, cx| input.set_value(text.to_owned(), window, cx));
            }
            Self::Area(area) => {
                area.update(cx, |area, cx| area.set_value(text.to_owned(), window, cx));
            }
        }
    }
}

pub struct SettingsView {
    source: DataSource,
    saved: Loaded<Value>,
    draft: Value,
    section: usize,
    editors: HashMap<&'static str, Editor>,
    errors: HashMap<&'static str, String>,
    user: User,
    saving: bool,
    _subscriptions: Vec<Subscription>,
}

fn get<'a>(root: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(root, |value, key| &value[key])
}

fn set(root: &mut Value, path: &str, new: Value) {
    let mut slot = root;
    for key in path.split('.') {
        slot = &mut slot[key];
    }
    *slot = new;
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn plain_decimal(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn editor_text(kind: Kind, value: &Value) -> String {
    match kind {
        Kind::Int { .. } => value.as_i64().map(|value| value.to_string()).unwrap_or_default(),
        Kind::Decimal { .. } => value.as_f64().map(plain_decimal).unwrap_or_default(),
        Kind::Words { .. } => strings(value).join(", "),
        _ => value.as_str().unwrap_or_default().to_owned(),
    }
}

fn parse(kind: Kind, text: &str) -> Result<Value, String> {
    match kind {
        Kind::Int { min, .. } => match text.trim().parse::<i64>() {
            Ok(number) if number < min => Err(format!("Enter {min} or more.")),
            Ok(number) => Ok(json!(number)),
            Err(_) => Err("Enter a whole number.".to_owned()),
        },
        Kind::Decimal { .. } => match text.trim().parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(json!(number)),
            _ => Err("Enter a number.".to_owned()),
        },
        Kind::Words { .. } => Ok(json!(
            text.split(',')
                .map(str::trim)
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
        )),
        _ => Ok(json!(text)),
    }
}

fn choice_label(choices: Choices, value: &str) -> String {
    choices
        .iter()
        .find(|(key, _)| *key == value)
        .map_or_else(|| value.to_owned(), |(_, label)| (*label).to_owned())
}

fn cell_text(kind: Kind, value: &Value) -> String {
    match kind {
        Kind::Toggle => if value.as_bool().unwrap_or(false) { "Yes" } else { "No" }.to_owned(),
        Kind::Flags(choices) => {
            let on = strings(value);
            let labels: Vec<_> = choices
                .iter()
                .filter(|(key, _)| on.iter().any(|flag| flag == key))
                .map(|(_, label)| *label)
                .collect();
            if labels.is_empty() {
                "None".to_owned()
            } else {
                labels.join(", ")
            }
        }
        Kind::Choice(choices) => choice_label(choices, value.as_str().unwrap_or_default()),
        Kind::Int { .. } => value
            .as_i64()
            .map(crate::ui::number)
            .unwrap_or_default(),
        _ => editor_text(kind, value),
    }
}

fn source_parts(value: &Value) -> (&'static str, String) {
    SOURCES
        .iter()
        .find_map(|(variant, _, _, key)| {
            let detail = value.get(*variant)?;
            Some((*variant, detail[*key].as_str().unwrap_or_default().to_owned()))
        })
        .unwrap_or(("None", String::new()))
}

fn source_value(variant: &str, detail: &str) -> Value {
    match SOURCES.iter().find(|(name, ..)| *name == variant) {
        Some((name, _, _, key)) if !key.is_empty() => json!({ *name: { *key: detail } }),
        _ => json!("None"),
    }
}

impl SettingsView {
    pub fn new(backend: &Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription =
            cx.subscribe_in(backend, window, |view, _, event: &BackendEvent, window, cx| {
                match event {
                    BackendEvent::Changed(DataKind::Settings) if !view.dirty() => {
                        view.load(window, cx);
                    }
                    BackendEvent::Changed(DataKind::User) => view.load_user(cx),
                    _ => {}
                }
            });
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            saved: Loaded::Loading,
            draft: Value::Null,
            section: 0,
            editors: HashMap::new(),
            errors: HashMap::new(),
            user: User::default(),
            saving: false,
            _subscriptions: vec![subscription],
        };
        view.load(window, cx);
        view.load_user(cx);
        view
    }

    pub fn open_section(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(index) = SECTIONS.iter().position(|section| section.key == key) {
            self.section = index;
            cx.notify();
        }
    }

    fn dirty(&self) -> bool {
        self.saved.ready().is_some_and(|saved| *saved != self.draft)
    }

    fn load_user(&mut self, cx: &mut Context<Self>) {
        let user = self.source.user();
        cx.spawn(async move |view, cx| {
            if let Ok(user) = user.await {
                view.update(cx, |view, cx| {
                    view.user = user;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let settings = self.source.app_get_settings();
        cx.spawn_in(window, async move |view, cx| {
            let settings = settings.await;
            view.update_in(cx, |view, window, cx| {
                view.stored(settings.map_err(|error| kit::describe(&error)), window, cx);
            })
            .ok();
        })
        .detach();
    }

    fn stored(&mut self, settings: Result<Settings, String>, window: &mut Window, cx: &mut Context<Self>) {
        let value = settings
            .and_then(|settings| serde_json::to_value(settings).map_err(|error| error.to_string()));
        match value {
            Ok(value) => {
                self.draft = value.clone();
                self.saved = Loaded::Ready(value);
                self.errors.clear();
                self.sync_editors(window, cx);
            }
            Err(reason) => self.saved = Loaded::Failed(reason),
        }
        cx.notify();
    }

    fn fields() -> impl Iterator<Item = Field> {
        SECTIONS
            .iter()
            .flat_map(|section| section.blocks.iter())
            .flat_map(|(_, fields)| fields.iter().copied())
    }

    fn sync_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for field in Self::fields() {
            let placeholder = match field.kind {
                Kind::Text { placeholder } | Kind::Words { placeholder } => placeholder,
                Kind::Int { .. } | Kind::Decimal { .. } => "",
                Kind::Long => "",
                Kind::Toggle | Kind::Choice(_) | Kind::Flags(_) => continue,
            };
            let text = editor_text(field.kind, get(&self.draft, field.path));
            self.sync_editor(field.path, Some(field), placeholder, &text, window, cx);
        }
        let (_, detail) = source_parts(get(&self.draft, SOURCE_PATH));
        self.sync_editor(SOURCE_DETAIL, None, "", &detail, window, cx);
    }

    fn sync_editor(
        &mut self,
        key: &'static str,
        field: Option<Field>,
        placeholder: &'static str,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.editors.get(key) {
            return editor.set_text(text, window, cx);
        }
        let editor = if matches!(field.map(|field| field.kind), Some(Kind::Long)) {
            let area = cx.new(|cx| {
                let mut area = TextareaState::new(window, cx).rows(4);
                area.set_value(text.to_owned(), window, cx);
                area
            });
            self._subscriptions
                .push(cx.subscribe(&area, move |view, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        view.edited(key, field, cx);
                    }
                }));
            Editor::Area(area)
        } else {
            let input = cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder);
                input.set_value(text.to_owned(), window, cx);
                input
            });
            self._subscriptions
                .push(cx.subscribe(&input, move |view, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        view.edited(key, field, cx);
                    }
                }));
            Editor::Line(input)
        };
        self.editors.insert(key, editor);
    }

    fn edited(&mut self, key: &'static str, field: Option<Field>, cx: &mut Context<Self>) {
        let Some(text) = self.editors.get(key).map(|editor| editor.text(cx)) else {
            return;
        };
        let Some(field) = field else {
            let (variant, detail) = source_parts(get(&self.draft, SOURCE_PATH));
            if detail != text {
                set(&mut self.draft, SOURCE_PATH, source_value(variant, &text));
                cx.notify();
            }
            return;
        };
        if editor_text(field.kind, get(&self.draft, field.path)) == text {
            self.errors.remove(key);
        } else {
            match parse(field.kind, &text) {
                Ok(value) => {
                    set(&mut self.draft, field.path, value);
                    self.errors.remove(key);
                }
                Err(reason) => {
                    self.errors.insert(key, reason);
                }
            }
        }
        cx.notify();
    }

    fn put(&mut self, path: &str, value: Value, cx: &mut Context<Self>) {
        set(&mut self.draft, path, value);
        cx.notify();
    }

    fn revert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(saved) = self.saved.ready().cloned() {
            self.draft = saved;
            self.errors.clear();
            self.sync_editors(window, cx);
            cx.notify();
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let settings = match serde_json::from_value::<Settings>(self.draft.clone()) {
            Ok(settings) => settings,
            Err(error) => {
                return window.push_notification(
                    Notification::error(error.to_string()).title("Could not save the settings"),
                    cx,
                );
            }
        };
        self.store(settings, "Saved the settings.", window, cx);
    }

    fn store(
        &mut self,
        settings: Settings,
        notice: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.saving = true;
        let stored = self.source.app_update_settings(settings);
        cx.spawn_in(window, async move |view, cx| {
            let stored = stored.await;
            view.update_in(cx, |view, window, cx| {
                view.saving = false;
                match stored {
                    Ok(settings) => {
                        view.stored(Ok(settings), window, cx);
                        window.push_notification(notice, cx);
                    }
                    Err(error) => window.push_notification(
                        Notification::error(kit::describe(&error))
                            .title("Could not save the settings"),
                        cx,
                    ),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn ask_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        form::open(
            FormSpec::confirm_danger(
                "Reset every setting?",
                "This puts every setting back to its default, including the blacklist, the buy list and the notification texts. You can't undo this.",
                "Reset",
            ),
            move |_, window, cx| {
                view.update(cx, |view, cx| view.reset(window, cx)).ok();
            },
            window,
            cx,
        );
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let defaults = self.source.app_get_default_settings();
        cx.spawn_in(window, async move |view, cx| {
            let defaults = defaults.await;
            view.update_in(cx, |view, window, cx| match defaults {
                Ok(defaults) => view.store(defaults, "Reset every setting.", window, cx),
                Err(error) => window.push_notification(
                    Notification::error(kit::describe(&error))
                        .title("Could not read the default settings"),
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    fn run<T: 'static>(
        &mut self,
        call: impl Future<Output = crate::data::DataResult<T>> + Send + 'static,
        notice: impl FnOnce(&T) -> String + 'static,
        failure: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |view, cx| {
            let result = call.await;
            view.update_in(cx, |view, window, cx| {
                match result {
                    Ok(value) => window.push_notification(notice(&value), cx),
                    Err(error) => window.push_notification(
                        Notification::error(kit::describe(&error)).title(failure),
                        cx,
                    ),
                }
                view.load_user(cx);
            })
            .ok();
        })
        .detach();
    }

    fn popup(
        &self,
        id: impl Into<gpui::ElementId>,
        choices: Vec<(SharedString, SharedString)>,
        current: &str,
        on_pick: impl Fn(&mut Self, &str, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = choices
            .iter()
            .find(|(key, _)| key.as_ref() == current)
            .map_or_else(|| current.to_owned(), |(_, label)| label.to_string());
        let current = current.to_owned();
        let view = cx.entity().downgrade();
        let on_pick = std::rc::Rc::new(on_pick);
        text_button(id, &label, cx)
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                for (key, label) in &choices {
                    let (view, on_pick, key) = (view.clone(), on_pick.clone(), key.clone());
                    menu = menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(key.as_ref() == current)
                            .on_click(move |_, window, cx| {
                                view.update(cx, |view, cx| on_pick(view, &key, window, cx))
                                    .ok();
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn control(&self, field: Field, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let value = get(&self.draft, field.path);
        let path = field.path;
        let unit = |unit: &'static str| {
            div()
                .pr(px(4.))
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(colors.text_faint)
                .child(unit)
        };
        let line = |width: f32, suffix: &'static str| match self.editors.get(path) {
            Some(Editor::Line(input)) => {
                let input = Input::new(input);
                div()
                    .w(px(width))
                    .child(if suffix.is_empty() {
                        input
                    } else {
                        input.suffix(unit(suffix))
                    })
                    .into_any_element()
            }
            _ => div().into_any_element(),
        };
        match field.kind {
            Kind::Toggle => Switch::new(SharedString::from(path))
                .checked(value.as_bool().unwrap_or(false))
                .on_click(cx.listener(move |view, checked: &bool, _, cx| {
                    view.put(path, json!(*checked), cx);
                }))
                .into_any_element(),
            Kind::Int { unit, .. } | Kind::Decimal { unit } => line(170., unit),
            Kind::Text { .. } | Kind::Words { .. } => line(320., ""),
            Kind::Long => match self.editors.get(path) {
                Some(Editor::Area(area)) => Textarea::new(area).into_any_element(),
                _ => div().into_any_element(),
            },
            Kind::Choice(choices) => self.popup(
                SharedString::from(path),
                choices
                    .iter()
                    .map(|(key, label)| ((*key).into(), (*label).into()))
                    .collect(),
                value.as_str().unwrap_or_default(),
                move |view, key, _, cx| view.put(path, json!(key), cx),
                cx,
            ),
            Kind::Flags(choices) => {
                let on = strings(value);
                div()
                    .flex()
                    .gap(px(6.))
                    .children(choices.iter().map(|(key, label)| {
                        let selected = on.iter().any(|flag| flag == key);
                        chip(SharedString::from(format!("{path}-{key}")), selected, cx)
                            .child(*label)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let mut flags = strings(get(&view.draft, path));
                                if selected {
                                    flags.retain(|flag| flag != key);
                                } else {
                                    flags.push((*key).to_owned());
                                }
                                view.put(path, json!(flags), cx);
                            }))
                    }))
                    .into_any_element()
            }
        }
    }

    fn row(&self, field: Field, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let error = self.errors.get(field.path).cloned();
        let text = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(div().text_color(colors.text).child(field.label))
            .when(!field.hint.is_empty(), |text| {
                text.child(kit::note(field.hint, cx))
            })
            .children(error.map(|error| {
                div()
                    .text_size(theme.type_scale.caption + px(1.))
                    .text_color(colors.negative)
                    .child(error)
            }));
        let control = self.control(field, cx);
        let row = div().px(px(14.)).py(px(10.)).min_h(px(46.));
        if matches!(field.kind, Kind::Long) {
            row.flex().flex_col().gap(px(8.)).child(text).child(control)
        } else {
            row.flex()
                .items_center()
                .gap(px(20.))
                .child(text)
                .child(div().flex_shrink_0().child(control))
        }
    }

    fn group(title: &str, rows: Vec<Div>, cx: &App) -> Div {
        let mut card = kit::card(cx);
        for (index, row) in rows.into_iter().enumerate() {
            if index > 0 {
                card = card.child(kit::rule(cx));
            }
            card = card.child(row);
        }
        div()
            .flex()
            .flex_col()
            .gap(px(7.))
            .when(!title.is_empty(), |group| {
                group.child(div().px(px(2.)).child(kit::overline(title.to_owned(), cx)))
            })
            .child(card)
    }

    fn plain_row(label: &str, detail: &str, control: impl IntoElement, cx: &App) -> Div {
        let colors = Theme::global(cx).colors;
        div()
            .flex()
            .items_center()
            .gap(px(20.))
            .px(px(14.))
            .py(px(10.))
            .min_h(px(46.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_color(colors.text).child(label.to_owned()))
                    .when(!detail.is_empty(), |text| {
                        text.child(kit::note(detail.to_owned(), cx))
                    }),
            )
            .child(div().flex_shrink_0().child(control))
    }

    fn list_form(spec: &ListSpec, entry: &Value) -> FormSpec {
        let editing = !entry.is_null();
        let mut form = FormSpec::new(
            if editing {
                format!("Edit the {}", spec.noun)
            } else {
                format!("Add a {}", spec.noun)
            },
            if editing { "Done" } else { "Add" },
        )
        .description("The change is saved with the other settings.");
        for column in spec.columns {
            let value = &entry[column.path];
            let fields = match column.kind {
                Kind::Int { min, .. } => vec![
                    FormField::number(column.path, column.label, value.as_i64().or(Some(0)))
                        .min(min),
                ],
                Kind::Toggle => vec![FormField::toggle(
                    column.path,
                    column.label,
                    value.as_bool().unwrap_or(false),
                )],
                Kind::Flags(choices) => {
                    let on = strings(value);
                    choices
                        .iter()
                        .map(|(key, label)| {
                            FormField::toggle(
                                key,
                                &format!("{}: {label}", column.label),
                                !editing || on.iter().any(|flag| flag == key),
                            )
                        })
                        .collect()
                }
                Kind::Words { placeholder } => vec![
                    FormField::text(
                        column.path,
                        column.label,
                        &strings(value).join(", "),
                        placeholder,
                    )
                    .optional(),
                ],
                Kind::Text { placeholder } => vec![FormField::text(
                    column.path,
                    column.label,
                    value.as_str().unwrap_or_default(),
                    placeholder,
                )],
                Kind::Decimal { .. } | Kind::Long | Kind::Choice(_) => Vec::new(),
            };
            for field in fields {
                form = form.field(if column.hint.is_empty() {
                    field
                } else {
                    field.hint(column.hint)
                });
            }
        }
        form
    }

    fn list_entry(spec: &ListSpec, mut entry: Value, values: &FormValues) -> Value {
        if !entry.is_object() {
            entry = json!({});
        }
        for column in spec.columns {
            entry[column.path] = match column.kind {
                Kind::Int { .. } => json!(values.number(column.path).unwrap_or(0)),
                Kind::Toggle => json!(values.flag(column.path)),
                Kind::Flags(choices) => json!(
                    choices
                        .iter()
                        .filter(|(key, _)| values.flag(key))
                        .map(|(key, _)| *key)
                        .collect::<Vec<_>>()
                ),
                Kind::Words { .. } => json!(values.list(column.path)),
                _ => json!(values.text(column.path)),
            };
        }
        entry
    }

    fn edit_entry(
        &mut self,
        spec: ListSpec,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entry = index
            .and_then(|index| get(&self.draft, spec.path).get(index))
            .cloned()
            .unwrap_or(Value::Null);
        let view = cx.entity().downgrade();
        form::open(
            Self::list_form(&spec, &entry),
            move |values, _, cx| {
                let entry = Self::list_entry(&spec, entry.clone(), values);
                view.update(cx, |view, cx| {
                    let mut entries = get(&view.draft, spec.path)
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    match index {
                        Some(index) if index < entries.len() => entries[index] = entry,
                        _ => entries.push(entry),
                    }
                    view.put(spec.path, Value::Array(entries), cx);
                })
                .ok();
            },
            window,
            cx,
        );
    }

    fn list(&self, spec: ListSpec, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let entries = get(&self.draft, spec.path)
            .as_array()
            .cloned()
            .unwrap_or_default();
        let cell = |text: String| div().flex_1().min_w_0().truncate().child(text);
        let header = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(theme.metrics.header_height)
            .px(px(14.))
            .text_size(theme.type_scale.caption)
            .font_weight(theme.type_scale.label_weight)
            .text_color(colors.text_muted)
            .children(spec.columns.iter().map(|column| cell(column.label.to_owned())))
            .child(div().w(px(56.)).flex_shrink_0());
        let rows = entries.iter().enumerate().map(|(index, entry)| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .h(theme.metrics.row_height)
                .px(px(14.))
                .border_t_1()
                .border_color(colors.border)
                .font_features(tabular_figures())
                .children(spec.columns.iter().enumerate().map(|(column_ix, column)| {
                    cell(cell_text(column.kind, &entry[column.path])).text_color(
                        if column_ix == 0 {
                            colors.text
                        } else {
                            colors.text_muted
                        },
                    )
                }))
                .child(
                    div()
                        .w(px(56.))
                        .flex_shrink_0()
                        .flex()
                        .justify_end()
                        .child(
                            IconButton::new(
                                SharedString::from(format!("{}-edit-{index}", spec.path)),
                                IconName::SquarePen,
                                format!("Edit this {}", spec.noun),
                            )
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.edit_entry(spec, Some(index), window, cx);
                            })),
                        )
                        .child(
                            IconButton::new(
                                SharedString::from(format!("{}-remove-{index}", spec.path)),
                                IconName::Trash,
                                format!("Remove this {}", spec.noun),
                            )
                            .hover_tone(colors.negative)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let mut entries = get(&view.draft, spec.path)
                                    .as_array()
                                    .cloned()
                                    .unwrap_or_default();
                                if index < entries.len() {
                                    entries.remove(index);
                                }
                                view.put(spec.path, Value::Array(entries), cx);
                            })),
                        ),
                )
        });
        let card = kit::card(cx)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(20.))
                    .px(px(14.))
                    .py(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(kit::heading(spec.title, cx))
                            .child(kit::note(spec.hint, cx)),
                    )
                    .child(
                        text_button(SharedString::from(format!("{}-add", spec.path)), "Add", cx)
                            .icon(Icon::new(IconName::Plus).size(theme.metrics.icon))
                            .on_click(cx.listener(move |view, _, window, cx| {
                                view.edit_entry(spec, None, window, cx);
                            })),
                    ),
            )
            .child(kit::rule(cx));
        if entries.is_empty() {
            card.child(
                div()
                    .px(px(14.))
                    .py(px(12.))
                    .text_color(colors.text_faint)
                    .child(format!("No {} yet.", spec.noun)),
            )
        } else {
            card.child(header).children(rows)
        }
    }

    fn account(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let user = &self.user;
        if user.anonymous || user.wfm_username.is_empty() {
            return kit::card(cx)
                .p(px(14.))
                .child(kit::note("Nobody is signed in.", cx));
        }
        let facts = kit::card(cx)
            .px(px(14.))
            .py(px(8.))
            .child(kit::fact("Name", user.wfm_username.clone(), Tone::Plain, cx))
            .child(kit::fact(
                "Platform",
                user.platform.to_uppercase(),
                Tone::Plain,
                cx,
            ))
            .child(kit::fact("Language on warframe.market", user.locale.clone(), Tone::Plain, cx))
            .child(kit::fact(
                "Account verified",
                if user.verification { "Yes" } else { "No" },
                Tone::Plain,
                cx,
            ))
            .child(kit::fact(
                "Order limit",
                crate::ui::number(user.order_limit),
                Tone::Plain,
                cx,
            ));
        let statuses = [UserStatus::Online, UserStatus::InGame, UserStatus::Invisible];
        let status = self.popup(
            "account-status",
            statuses
                .iter()
                .map(|status| (status.as_str().into(), status.label().into()))
                .collect(),
            user.wfm_status.as_str(),
            move |view, key, window, cx| {
                let status = UserStatus::parse(key);
                view.run(
                    view.source.user_set_status(status),
                    move |_| format!("Your status is now {}.", status.label().to_lowercase()),
                    "Could not change the status",
                    window,
                    cx,
                );
            },
            cx,
        );
        let sign_out = text_button("sign-out", "Sign out", cx)
            .icon(Icon::new(IconName::LogOut).size(theme.metrics.icon))
            .on_click(cx.listener(|view, _, window, cx| {
                view.run(
                    view.source.auth_logout(),
                    |_| "Signed out.".to_owned(),
                    "Could not sign out",
                    window,
                    cx,
                );
            }));
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(facts)
            .child(Self::group(
                "",
                vec![
                    Self::plain_row(
                        "Status on warframe.market",
                        "Other traders see this beside your orders.",
                        status,
                        cx,
                    ),
                    Self::plain_row(
                        "Sign out",
                        "The live scraper needs an account, so it stops.",
                        sign_out,
                        cx,
                    ),
                ],
                cx,
            ))
    }

    fn inventory_source(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let (variant, _) = source_parts(get(&self.draft, SOURCE_PATH));
        let chosen = SOURCES.iter().find(|(name, ..)| *name == variant);
        let picker = self.popup(
            "inventory-source",
            SOURCES
                .iter()
                .map(|(name, label, ..)| ((*name).into(), (*label).into()))
                .collect(),
            variant,
            |view, key, _, cx| {
                let detail = view
                    .editors
                    .get(SOURCE_DETAIL)
                    .map(|editor| editor.text(cx))
                    .unwrap_or_default();
                view.put(SOURCE_PATH, source_value(key, &detail), cx);
            },
            cx,
        );
        let mut rows = vec![Self::plain_row(
            "Read the inventory from",
            match variant {
                "Profile" => "Your public profile on api.warframe.com.",
                "Alecaframe" => "AlecaFrame's data file. Leave the file empty for its default place.",
                "File" => "A JSON file in the lastData or InventoryJson format. Quantframe reads it again when it changes.",
                _ => "With no source, the Inventory screen stays empty.",
            },
            picker,
            cx,
        )];
        if let (Some((_, _, label, key)), Some(Editor::Line(input))) =
            (chosen, self.editors.get(SOURCE_DETAIL))
            && !key.is_empty()
        {
            rows.push(Self::plain_row(
                label,
                "",
                div().w(px(320.)).child(Input::new(input)),
                cx,
            ));
        }
        rows.push(Self::plain_row(
            "Read the inventory now",
            "Uses the saved source.",
            text_button("inventory-refresh", "Read now", cx)
                .icon(Icon::new(IconName::RefreshCw).size(theme.metrics.icon))
                .on_click(cx.listener(|view, _, window, cx| {
                    view.run(
                        view.source.wf_inventory_update(),
                        |_| "Read the inventory again.".to_owned(),
                        "Could not read the inventory",
                        window,
                        cx,
                    );
                })),
            cx,
        ));
        Self::group("", rows, cx)
    }

    fn extra(&self, extra: Extra, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        let theme = Theme::global(cx).clone();
        Some(match extra {
            Extra::None => return None,
            Extra::Account => self.account(cx),
            Extra::InventorySource => self.inventory_source(cx),
            Extra::Reset => Self::group(
                "",
                vec![Self::plain_row(
                    "Reset every setting",
                    "Puts every setting back to its default.",
                    text_button("reset-settings", "Reset", cx)
                        .danger()
                        .on_click(cx.listener(|view, _, window, cx| view.ask_reset(window, cx))),
                    cx,
                )],
                cx,
            ),
            Extra::Appearance => {
                let current = theme.choice;
                let system = window.appearance();
                Self::group(
                    "Window",
                    vec![Self::plain_row(
                        "Appearance",
                        "Saved on this computer at once, apart from the settings below.",
                        segmented(
                            "settings-appearance",
                            AppearanceChoice::ALL
                                .iter()
                                .map(|choice| Segment {
                                    label: choice.label(),
                                    count: None,
                                })
                                .collect(),
                            AppearanceChoice::ALL
                                .iter()
                                .position(|choice| *choice == current)
                                .unwrap_or(0),
                            move |index, _, cx| {
                                Theme::set_choice(AppearanceChoice::ALL[index], system, cx);
                            },
                            cx,
                        ),
                        cx,
                    )],
                    cx,
                )
            }
            Extra::Logs => Self::group(
                "",
                vec![Self::plain_row(
                    "Export Quantframe's logs",
                    "Writes a zip file to send with a bug report.",
                    text_button("export-logs", "Export logs", cx)
                        .icon(Icon::new(IconName::Download).size(theme.metrics.icon))
                        .on_click(cx.listener(|view, _, window, cx| {
                            view.run(
                                view.source.log_export(),
                                |path| format!("Wrote the logs to {}.", path.display()),
                                "Could not export the logs",
                                window,
                                cx,
                            );
                        })),
                    cx,
                )],
                cx,
            ),
            Extra::Analytics => Self::group(
                "Analytics",
                vec![
                    div().px(px(14.)).py(px(12.)).child(kit::note(
                        "While you are signed in and not banned, Quantframe reports how the app is used to its own server. The backend has no setting that turns the reports off.",
                        cx,
                    )),
                ],
                cx,
            ),
        })
    }

    fn nav(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let mut nav = div()
            .id("settings-nav")
            .w(px(NAV_WIDTH))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(1.))
            .px(px(8.))
            .py(px(8.))
            .overflow_y_scroll()
            .border_r_1()
            .border_color(colors.border);
        let mut group = "";
        for (index, section) in SECTIONS.iter().enumerate() {
            if section.group != group {
                group = section.group;
                nav = nav.child(
                    div()
                        .px(px(9.))
                        .pt(px(if index == 0 { 6. } else { 14. }))
                        .pb(px(5.))
                        .child(kit::overline(group, cx)),
                );
            }
            let active = index == self.section;
            let ring = focus_ring(&theme);
            nav = nav.child(
                div()
                    .id(section.key)
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(9.))
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(theme.metrics.radius)
                    .cursor_pointer()
                    .tab_index(0)
                    .focus_visible(move |style| style.shadow(ring))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.section = index;
                        cx.notify();
                    }))
                    .map(|item| {
                        if active {
                            item.bg(colors.row_selected)
                                .text_color(colors.text)
                                .font_weight(theme.type_scale.label_weight)
                        } else {
                            item.text_color(colors.text_muted)
                                .hover(|item| item.bg(colors.control_hover).text_color(colors.text))
                        }
                    })
                    .child(icon(
                        section.icon,
                        theme.metrics.icon,
                        if active {
                            colors.accent
                        } else {
                            colors.text_muted
                        },
                    ))
                    .child(section.title),
            );
        }
        nav
    }

    fn section_body(&self, section: &Section, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let mut body = div()
            .w_full()
            .max_w(px(CONTENT_WIDTH))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(theme.type_scale.title + px(2.))
                            .font_weight(theme.type_scale.title_weight)
                            .text_color(theme.colors.text)
                            .child(section.title),
                    )
                    .child(kit::note(section.blurb, cx)),
            );
        if matches!(section.extra, Extra::Account | Extra::Appearance | Extra::InventorySource)
            && let Some(extra) = self.extra(section.extra, window, cx)
        {
            body = body.child(extra);
        }
        for (title, fields) in section.blocks {
            let rows = fields.iter().map(|field| self.row(*field, cx)).collect();
            body = body.child(Self::group(title, rows, cx));
        }
        for spec in section.lists {
            body = body.child(self.list(*spec, cx));
        }
        if matches!(section.extra, Extra::Reset | Extra::Logs | Extra::Analytics)
            && let Some(extra) = self.extra(section.extra, window, cx)
        {
            body = body.child(extra);
        }
        body
    }

    fn save_bar(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let dirty = self.dirty();
        let invalid = !self.errors.is_empty();
        let (state, tone) = if invalid {
            ("Fix the marked fields before you save.", colors.negative)
        } else if dirty {
            ("You have unsaved changes.", colors.text)
        } else {
            ("Every change is saved.", colors.text_muted)
        };
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .h(px(52.))
            .px(theme.metrics.gutter + px(8.))
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.chrome)
            .child(div().text_color(tone).child(state))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(
                        text_button("settings-revert", "Revert", cx)
                            .disabled(!dirty && !invalid)
                            .on_click(cx.listener(|view, _, window, cx| view.revert(window, cx))),
                    )
                    .child(
                        text_button("settings-save", "Save changes", cx)
                            .primary()
                            .disabled(!dirty || invalid || self.saving)
                            .on_click(cx.listener(|view, _, window, cx| view.save(window, cx))),
                    ),
            )
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let root = div()
            .size_full()
            .flex()
            .bg(theme.colors.surface)
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text);
        match &self.saved {
            Loaded::Loading => root.child(placeholder(
                IconName::Settings,
                "Loading the settings",
                "Reading them from the backend.",
                cx,
            )),
            Loaded::Failed(reason) => root.child(kit::failed(
                "Could not load the settings",
                reason,
                cx.listener(|view, _, window, cx| view.load(window, cx)),
                cx,
            )),
            Loaded::Ready(_) => {
                let section = &SECTIONS[self.section];
                root.child(self.nav(cx)).child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id(section.key)
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .px(theme.metrics.gutter + px(8.))
                                .py(px(20.))
                                .child(self.section_body(section, window, cx)),
                        )
                        .child(self.save_bar(cx)),
                )
            }
        }
    }
}

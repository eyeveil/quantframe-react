//! Dialogs built from a list of fields: the sell, buy, edit, create and
//! confirm prompts of every screen.

use std::{cell::Cell, rc::Rc};

use gpui::{
    App, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    IndexPath, WindowExt,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    searchable_list::SearchableVec,
    select::{Select, SelectState},
    switch::Switch,
};

use super::text_button;
use crate::theme::Theme;

pub enum FieldKind {
    Number {
        value: Option<i64>,
        min: i64,
        max: Option<i64>,
        unit: Option<&'static str>,
    },
    Text {
        value: String,
        placeholder: &'static str,
    },
    Choice {
        options: Vec<String>,
        selected: Option<usize>,
    },
    Toggle {
        value: bool,
    },
}

pub struct Field {
    key: &'static str,
    label: String,
    hint: Option<String>,
    kind: FieldKind,
    optional: bool,
    half: bool,
}

impl Field {
    fn new(key: &'static str, label: &str, kind: FieldKind) -> Self {
        Self {
            key,
            label: label.to_owned(),
            hint: None,
            kind,
            optional: false,
            half: false,
        }
    }

    /// A whole number of zero or more.
    pub fn number(key: &'static str, label: &str, value: Option<i64>) -> Self {
        Self::new(
            key,
            label,
            FieldKind::Number {
                value,
                min: 0,
                max: None,
                unit: None,
            },
        )
    }

    /// A platinum amount of zero or more.
    pub fn platinum(key: &'static str, label: &str, value: Option<i64>) -> Self {
        Self::number(key, label, value).unit("platinum")
    }

    pub fn text(key: &'static str, label: &str, value: &str, placeholder: &'static str) -> Self {
        Self::new(
            key,
            label,
            FieldKind::Text {
                value: value.to_owned(),
                placeholder,
            },
        )
    }

    pub fn choice(
        key: &'static str,
        label: &str,
        options: Vec<String>,
        selected: Option<usize>,
    ) -> Self {
        Self::new(key, label, FieldKind::Choice { options, selected })
    }

    pub fn toggle(key: &'static str, label: &str, value: bool) -> Self {
        Self::new(key, label, FieldKind::Toggle { value })
    }

    pub fn min(mut self, minimum: i64) -> Self {
        if let FieldKind::Number { min, .. } = &mut self.kind {
            *min = minimum;
        }
        self
    }

    pub fn max(mut self, maximum: i64) -> Self {
        if let FieldKind::Number { max, .. } = &mut self.kind {
            *max = Some(maximum);
        }
        self
    }

    pub fn unit(mut self, text: &'static str) -> Self {
        if let FieldKind::Number { unit, .. } = &mut self.kind {
            *unit = Some(text);
        }
        self
    }

    /// Lets the field stay empty.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn hint(mut self, hint: &str) -> Self {
        self.hint = Some(hint.to_owned());
        self
    }

    /// Takes half the dialog's width, so two fields share a line.
    pub fn half(mut self) -> Self {
        self.half = true;
        self
    }
}

/// Text computed from the current values, shown under the fields.
pub struct Preview {
    pub label: &'static str,
    pub text: String,
    /// The longest text the destination accepts.
    pub limit: Option<usize>,
}

type PreviewBuilder = Rc<dyn Fn(&FormValues) -> Preview>;

pub struct FormSpec {
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<Field>,
    pub confirm: String,
    pub danger: bool,
    pub width: f32,
    pub preview: Option<PreviewBuilder>,
}

impl FormSpec {
    pub fn new(title: impl Into<String>, confirm: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            fields: Vec::new(),
            confirm: confirm.into(),
            danger: false,
            width: 420.,
            preview: None,
        }
    }

    /// A prompt with no fields that asks before a destructive change.
    pub fn confirm_danger(
        title: impl Into<String>,
        description: impl Into<String>,
        confirm: impl Into<String>,
    ) -> Self {
        let mut spec = Self::new(title, confirm).description(description);
        spec.danger = true;
        spec
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn preview(mut self, preview: impl Fn(&FormValues) -> Preview + 'static) -> Self {
        self.preview = Some(Rc::new(preview));
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(Option<i64>),
    Text(String),
    Choice(Option<String>),
    Flag(bool),
}

/// The values of a submitted form, by field key.
#[derive(Clone, Debug, Default)]
pub struct FormValues(Vec<(&'static str, Value)>);

impl FormValues {
    fn get(&self, key: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value)
    }

    pub fn number(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(Value::Number(value)) => *value,
            _ => None,
        }
    }

    pub fn text(&self, key: &str) -> String {
        match self.get(key) {
            Some(Value::Text(value)) => value.trim().to_owned(),
            _ => String::new(),
        }
    }

    pub fn choice(&self, key: &str) -> Option<String> {
        match self.get(key) {
            Some(Value::Choice(value)) => value.clone(),
            _ => None,
        }
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Value::Flag(true)))
    }

    /// The comma-separated entries of a text field, without blanks.
    pub fn list(&self, key: &str) -> Vec<String> {
        self.text(key)
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

enum Control {
    Input(Entity<InputState>),
    Select(Entity<SelectState<SearchableVec<SharedString>>>),
    Toggle(bool),
}

type EnterHandler = Rc<dyn Fn(&mut Window, &mut App)>;
type SubmitHandler = Rc<dyn Fn(&mut Window, &mut App) -> Submitted>;

struct Form {
    spec: FormSpec,
    controls: Vec<Control>,
    errors: Vec<Option<String>>,
    on_enter: Option<EnterHandler>,
    _subscriptions: Vec<Subscription>,
}

impl Form {
    fn new(spec: FormSpec, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        let controls = spec
            .fields
            .iter()
            .map(|field| match &field.kind {
                FieldKind::Number { value, .. } => {
                    let text = value.map(|value| value.to_string()).unwrap_or_default();
                    Control::Input(text_input(&text, "", &mut subscriptions, window, cx))
                }
                FieldKind::Text { value, placeholder } => {
                    Control::Input(text_input(value, placeholder, &mut subscriptions, window, cx))
                }
                FieldKind::Choice { options, selected } => {
                    let options: Vec<SharedString> =
                        options.iter().cloned().map(SharedString::from).collect();
                    let searchable = options.len() > 8;
                    let selected = selected.map(IndexPath::new);
                    Control::Select(cx.new(|cx| {
                        SelectState::new(SearchableVec::new(options), selected, window, cx)
                            .searchable(searchable)
                    }))
                }
                FieldKind::Toggle { value } => Control::Toggle(*value),
            })
            .collect::<Vec<_>>();
        Self {
            errors: vec![None; controls.len()],
            spec,
            controls,
            on_enter: None,
            _subscriptions: subscriptions,
        }
    }

    fn focus_first(&self, window: &mut Window, cx: &mut App) {
        let first = self.controls.iter().find_map(|control| match control {
            Control::Input(input) => Some(input.focus_handle(cx)),
            Control::Select(_) | Control::Toggle(_) => None,
        });
        if let Some(handle) = first {
            handle.focus(window, cx);
        }
    }

    /// Reads every field. A field that fails its check keeps its last good
    /// reading out of the result, and `strict` records why it failed.
    fn read(&mut self, strict: bool, cx: &App) -> Option<FormValues> {
        let mut values = Vec::with_capacity(self.controls.len());
        let mut valid = true;
        for (index, (field, control)) in self.spec.fields.iter().zip(&self.controls).enumerate() {
            let (value, error) = match (control, &field.kind) {
                (Control::Input(input), FieldKind::Number { min, max, .. }) => {
                    let text = input.read(cx).value();
                    let text = text.trim();
                    match text.parse::<i64>() {
                        _ if text.is_empty() && field.optional => (Value::Number(None), None),
                        _ if text.is_empty() => {
                            (Value::Number(None), Some("Enter a number.".to_owned()))
                        }
                        Ok(number) if number < *min => {
                            (Value::Number(None), Some(format!("Enter {min} or more.")))
                        }
                        Ok(number) if max.is_some_and(|max| number > max) => (
                            Value::Number(None),
                            max.map(|max| format!("Enter {max} or less.")),
                        ),
                        Ok(number) => (Value::Number(Some(number)), None),
                        Err(_) => (
                            Value::Number(None),
                            Some("Enter a whole number.".to_owned()),
                        ),
                    }
                }
                (Control::Input(input), _) => {
                    let text = input.read(cx).value().to_string();
                    let missing = text.trim().is_empty() && !field.optional;
                    (
                        Value::Text(text),
                        missing.then(|| "Fill in this field.".to_owned()),
                    )
                }
                (Control::Select(select), _) => {
                    let choice = select
                        .read(cx)
                        .selected_value()
                        .map(|value| value.to_string());
                    let missing = choice.is_none() && !field.optional;
                    (
                        Value::Choice(choice),
                        missing.then(|| "Choose one.".to_owned()),
                    )
                }
                (Control::Toggle(value), _) => (Value::Flag(*value), None),
            };
            valid &= error.is_none();
            if strict {
                self.errors[index] = error;
            }
            values.push((field.key, value));
        }
        (valid || !strict).then_some(FormValues(values))
    }

    fn render_field(&self, index: usize, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let field = &self.spec.fields[index];
        let caption = |text: String, color| {
            div()
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(color)
                .child(text)
        };
        let control = match (&self.controls[index], &field.kind) {
            (Control::Input(input), FieldKind::Number { unit, .. }) => {
                let input = Input::new(input);
                match unit {
                    Some(unit) => input
                        .suffix(caption((*unit).to_owned(), colors.text_faint).pr(px(4.)))
                        .into_any_element(),
                    None => input.into_any_element(),
                }
            }
            (Control::Input(input), _) => Input::new(input).into_any_element(),
            (Control::Select(select), _) => Select::new(select)
                .placeholder("Choose")
                .w_full()
                .into_any_element(),
            (Control::Toggle(value), _) => Switch::new(("toggle", index))
                .checked(*value)
                .label(field.label.clone())
                .on_click(cx.listener(move |form, checked: &bool, _, cx| {
                    form.controls[index] = Control::Toggle(*checked);
                    cx.notify();
                }))
                .into_any_element(),
        };
        let is_toggle = matches!(field.kind, FieldKind::Toggle { .. });
        div()
            .flex()
            .flex_col()
            .gap(px(5.))
            .when(field.half, |cell| cell.w(gpui::relative(0.5)).flex_shrink(1.))
            .when(!field.half, |cell| cell.w_full())
            .min_w_0()
            .px(px(5.))
            .when(!is_toggle, |cell| {
                cell.child(
                    div()
                        .text_size(theme.type_scale.caption + px(1.))
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(colors.text_muted)
                        .child(field.label.clone()),
                )
            })
            .child(control)
            .children(match (&self.errors[index], &field.hint) {
                (Some(error), _) => Some(caption(error.clone(), colors.negative)),
                (None, Some(hint)) => Some(caption(hint.clone(), colors.text_faint)),
                (None, None) => None,
            })
    }
}

fn text_input(
    value: &str,
    placeholder: &'static str,
    subscriptions: &mut Vec<Subscription>,
    window: &mut Window,
    cx: &mut Context<Form>,
) -> Entity<InputState> {
    let input = cx.new(|cx| {
        let mut state = InputState::new(window, cx).placeholder(placeholder);
        state.set_value(value.to_owned(), window, cx);
        state
    });
    subscriptions.push(cx.subscribe_in(
        &input,
        window,
        |form, _, event: &InputEvent, window, cx| match event {
            InputEvent::Change => cx.notify(),
            InputEvent::PressEnter { .. } => {
                if let Some(on_enter) = form.on_enter.clone() {
                    window.defer(cx, move |window, cx| on_enter(window, cx));
                }
            }
            InputEvent::Focus | InputEvent::Blur => {}
        },
    ));
    input
}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let preview = self.spec.preview.clone().and_then(|preview| {
            self.read(false, cx).map(|values| preview(&values))
        });
        let fields = (0..self.controls.len())
            .map(|index| self.render_field(index, cx))
            .collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .pt(px(6.))
            .pb(px(4.))
            .text_size(theme.type_scale.body)
            .text_color(colors.text)
            .children(self.spec.description.clone().map(|description| {
                div()
                    .text_color(colors.text_muted)
                    .line_height(px(19.))
                    .child(description)
            }))
            .when(!fields.is_empty(), |form| {
                form.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap_y(px(12.))
                        .mx(px(-5.))
                        .children(fields),
                )
            })
            .children(preview.map(|preview| {
                let length = preview.text.chars().count();
                let over = preview.limit.is_some_and(|limit| length > limit);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_size(theme.type_scale.caption + px(1.))
                            .child(
                                div()
                                    .font_weight(theme.type_scale.label_weight)
                                    .text_color(colors.text_muted)
                                    .child(preview.label),
                            )
                            .children(preview.limit.map(|limit| {
                                div()
                                    .text_color(if over {
                                        colors.negative
                                    } else {
                                        colors.text_faint
                                    })
                                    .child(format!("{length} of {limit} characters"))
                            })),
                    )
                    .child(
                        div()
                            .p(px(10.))
                            .min_h(px(54.))
                            .rounded(theme.metrics.radius)
                            .border_1()
                            .border_color(colors.border)
                            .bg(colors.window)
                            .line_height(px(19.))
                            .child(preview.text),
                    )
            }))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Submitted {
    Now,
    Earlier,
    Invalid,
}

/// Opens a dialog for `spec`. `on_submit` runs once, with the checked
/// values, when the user confirms.
pub fn open(
    spec: FormSpec,
    on_submit: impl Fn(&FormValues, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    let title = SharedString::from(spec.title.clone());
    let confirm = spec.confirm.clone();
    let danger = spec.danger;
    let width = spec.width;
    let form = cx.new(|cx| Form::new(spec, window, cx));

    let done = Cell::new(false);
    let on_submit = Rc::new(on_submit);
    let submit: SubmitHandler = {
        let form = form.clone();
        Rc::new(move |window, cx| {
            if done.get() {
                return Submitted::Earlier;
            }
            let values = form.update(cx, |form, cx| {
                let values = form.read(true, cx);
                cx.notify();
                values
            });
            let Some(values) = values else {
                return Submitted::Invalid;
            };
            done.set(true);
            // The dialog layer may still be updating when the keyboard
            // confirms, so the handler runs once that update has finished.
            let on_submit = on_submit.clone();
            window.defer(cx, move |window, cx| on_submit(&values, window, cx));
            Submitted::Now
        })
    };
    let submit_and_close: EnterHandler = {
        let submit = submit.clone();
        Rc::new(move |window, cx| {
            if submit(window, cx) == Submitted::Now {
                window.close_dialog(cx);
            }
        })
    };
    form.update(cx, |form, _| form.on_enter = Some(submit_and_close.clone()));

    let first_build = Cell::new(true);
    window.open_dialog(cx, move |dialog, window, cx| {
        if first_build.replace(false) {
            form.update(cx, |form, cx| form.focus_first(window, cx));
        }
        let on_ok = submit.clone();
        let on_confirm = submit_and_close.clone();
        let confirm_button = text_button("form-confirm", &confirm, cx);
        let confirm_button = if danger {
            confirm_button.danger()
        } else {
            confirm_button.primary()
        };
        dialog
            .title(title.clone())
            .w(px(width))
            .child(form.clone())
            .on_ok(move |_, window, cx| on_ok(window, cx) != Submitted::Invalid)
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(
                        text_button("form-cancel", "Cancel", cx)
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        confirm_button.on_click(move |_, window, cx| on_confirm(window, cx)),
                    ),
            )
    });
}

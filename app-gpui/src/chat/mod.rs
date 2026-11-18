//! warframe.market conversations: the room list, the messages of one room
//! and the box to write a message.

use gpui::{
    App, Context, Div, Entity, Focusable, Hsla, IntoElement, ParentElement, Render, ScrollHandle,
    Stateful, Styled, Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    Disableable, Icon, WindowExt,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    notification::Notification,
};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, BackendEvent, DataKind, DataSource},
    kit::{self, Loaded},
    model::{Chat, ChatMessage, ChatUser, ListParams, SortDirection},
    theme::{Theme, tabular_figures},
    ui::{
        IconButton, day_and_time, dot, focus_ring,
        form::{self, FormSpec},
        number, placeholder, search_field, text_button,
    },
};

/// The longest message warframe.market accepts.
const MESSAGE_LIMIT: usize = 400;
const LIST_WIDTH: f32 = 320.;

pub struct ChatView {
    source: DataSource,
    /// The signed-in user's warframe.market id, to tell sent from received.
    me: String,
    chats: Loaded<Vec<Chat>>,
    active: Option<String>,
    /// The messages of the active chat, oldest first.
    messages: Loaded<Vec<ChatMessage>>,
    search: Entity<InputState>,
    composer: Entity<InputState>,
    sending: bool,
    scroll: ScrollHandle,
    /// The row to open and the action to run once the chats have loaded.
    on_load: Option<(Option<usize>, Option<String>)>,
    /// Set by a scripted screenshot: write a sample message and send it.
    send_sample: bool,
    _subscriptions: Vec<Subscription>,
}

fn status_label(status: &str) -> &'static str {
    match status {
        "online" => "Online",
        "ingame" | "in_game" => "In game",
        _ => "Offline",
    }
}

fn status_color(status: &str, theme: &Theme) -> Hsla {
    match status {
        "online" => theme.colors.positive,
        "ingame" | "in_game" => theme.colors.accent,
        _ => theme.colors.text_faint,
    }
}

/// The other people in a chat.
fn partners<'a>(chat: &'a Chat, me: &'a str) -> impl Iterator<Item = &'a ChatUser> {
    chat.chat_with.iter().filter(move |user| user.id != me)
}

impl ChatView {
    pub fn new(backend: &Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search conversations"));
        let composer = cx.new(|cx| InputState::new(window, cx).placeholder("Write a message"));
        let subscriptions = vec![
            cx.subscribe(backend, |view, _, event: &BackendEvent, cx| match event {
                BackendEvent::ChatMessage(message) => view.receive(message.clone(), cx),
                BackendEvent::Changed(DataKind::Chats) => view.load(cx),
                BackendEvent::Changed(DataKind::User) => view.load_user(cx),
                _ => {}
            }),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &composer,
                window,
                |view, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => view.send(window, cx),
                    InputEvent::Change => cx.notify(),
                    InputEvent::Focus | InputEvent::Blur => {}
                },
            ),
        ];
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            me: String::new(),
            chats: Loaded::Loading,
            active: None,
            messages: Loaded::Loading,
            search,
            composer,
            sending: false,
            scroll: ScrollHandle::new(),
            on_load: None,
            send_sample: false,
            _subscriptions: subscriptions,
        };
        view.load_user(cx);
        view.load(cx);
        view
    }

    /// Opens a chat and runs an action once the chats have loaded. Scripted
    /// screenshots use it: `send` writes a message and sends it.
    pub fn on_load(&mut self, row: Option<usize>, action: Option<String>) {
        self.on_load = Some((row, action));
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.composer.focus_handle(cx).focus(window, cx);
    }

    /// Tells the backend no chat is on screen, so new messages count as unread.
    pub fn leave(&self, cx: &mut App) {
        cx.background_spawn(self.source.chat_set_active(None)).detach();
    }

    /// Tells the backend which chat is on screen again.
    pub fn enter(&self, cx: &mut App) {
        cx.background_spawn(self.source.chat_set_active(self.active.clone()))
            .detach();
    }

    fn load_user(&mut self, cx: &mut Context<Self>) {
        let user = self.source.user();
        cx.spawn(async move |view, cx| {
            if let Ok(user) = user.await {
                view.update(cx, |view, cx| {
                    view.me = user.wfm_id;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let chats = self.source.get_chat_pagination(&ListParams {
            limit: -1,
            sort_by: Some("last_update".to_owned()),
            sort_direction: SortDirection::Desc,
            ..ListParams::default()
        });
        cx.spawn(async move |view, cx| {
            let chats = chats.await;
            view.update(cx, |view, cx| view.loaded(chats.map(|page| page.results), cx))
                .ok();
        })
        .detach();
    }

    fn loaded(&mut self, chats: crate::data::DataResult<Vec<Chat>>, cx: &mut Context<Self>) {
        self.chats = Loaded::from_result(chats);
        let chats = self.chats.ready().map(Vec::as_slice).unwrap_or_default();
        let (row, action) = self.on_load.take().unwrap_or_default();
        let wanted = match &self.active {
            Some(id) if chats.iter().any(|chat| chat.id == *id) => None,
            _ => chats.get(row.unwrap_or(0)).or(chats.first()),
        }
        .map(|chat| chat.id.clone());
        if self.active.as_ref().is_some_and(|id| !chats.iter().any(|chat| chat.id == *id)) {
            self.active = None;
        }
        if let Some(id) = wanted {
            self.open(id, cx);
        }
        self.send_sample = action.as_deref() == Some("send");
        cx.notify();
    }

    fn open(&mut self, id: String, cx: &mut Context<Self>) {
        if let Loaded::Ready(chats) = &mut self.chats
            && let Some(chat) = chats.iter_mut().find(|chat| chat.id == id)
        {
            chat.unread_count = 0;
        }
        self.active = Some(id.clone());
        self.messages = Loaded::Loading;
        cx.background_spawn(self.source.chat_set_active(Some(id.clone())))
            .detach();
        let messages = self.source.chat_get_messages_by_id(id.clone());
        cx.spawn(async move |view, cx| {
            let messages = messages.await;
            view.update(cx, |view, cx| {
                if view.active.as_deref() != Some(id.as_str()) {
                    return;
                }
                view.messages = Loaded::from_result(messages.map(|mut messages| {
                    messages.sort_by(|a, b| a.send_date.cmp(&b.send_date));
                    messages
                }));
                view.scroll.scroll_to_bottom();
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn receive(&mut self, message: ChatMessage, cx: &mut Context<Self>) {
        let on_screen = self.active.as_deref() == Some(message.chat_id.as_str());
        let Loaded::Ready(chats) = &mut self.chats else {
            return;
        };
        let Some(chat) = chats.iter_mut().find(|chat| chat.id == message.chat_id) else {
            return self.load(cx);
        };
        chat.last_update = message.send_date.clone();
        chat.last_message = Some(message.clone());
        if on_screen {
            if let Loaded::Ready(messages) = &mut self.messages
                && !messages.iter().any(|known| known.id == message.id)
            {
                messages.push(message);
                self.scroll.scroll_to_bottom();
            }
        } else {
            chat.unread_count += 1;
        }
        chats.sort_by(|a, b| b.last_update.cmp(&a.last_update));
        cx.notify();
    }

    fn draft(&self, cx: &App) -> String {
        self.composer.read(cx).value().trim().to_owned()
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.draft(cx);
        let Some(id) = self.active.clone() else {
            return;
        };
        if self.sending || text.is_empty() || text.chars().count() > MESSAGE_LIMIT {
            return;
        }
        self.sending = true;
        let sent = self.source.chat_send_message(id, text);
        cx.spawn_in(window, async move |view, cx| {
            let sent = sent.await;
            view.update_in(cx, |view, window, cx| {
                view.sending = false;
                match sent {
                    Ok(()) => view
                        .composer
                        .update(cx, |composer, cx| composer.set_value("", window, cx)),
                    Err(error) => window.push_notification(
                        Notification::error(kit::describe(&error))
                            .title("The message was not sent")
                            .autohide(false),
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

    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let refreshed = self.source.chat_refresh();
        cx.spawn_in(window, async move |view, cx| {
            let refreshed = refreshed.await;
            view.update_in(cx, |view, window, cx| match refreshed {
                Ok(()) => view.load(cx),
                Err(error) => window.push_notification(
                    Notification::error(kit::describe(&error))
                        .title("Could not refresh the conversations"),
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    fn ask_delete(&mut self, id: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        form::open(
            FormSpec::confirm_danger(
                format!("Delete the conversation with {name}?"),
                "This deletes it on warframe.market too. You can't undo this.",
                "Delete",
            ),
            move |_, window, cx| {
                view.update(cx, |view, cx| view.delete(id.clone(), window, cx))
                    .ok();
            },
            window,
            cx,
        );
    }

    fn delete(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let deleted = self.source.chat_delete(id.clone());
        cx.spawn_in(window, async move |view, cx| {
            let deleted = deleted.await;
            view.update_in(cx, |view, window, cx| match deleted {
                Ok(()) => {
                    if view.active.as_deref() == Some(id.as_str()) {
                        view.active = None;
                    }
                    window.push_notification("Deleted the conversation.", cx);
                    view.load(cx);
                }
                Err(error) => window.push_notification(
                    Notification::error(kit::describe(&error))
                        .title("Could not delete the conversation"),
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    fn active_chat(&self) -> Option<&Chat> {
        let id = self.active.as_deref()?;
        self.chats.ready()?.iter().find(|chat| chat.id == id)
    }

    fn room(&self, chat: &Chat, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let selected = self.active.as_deref() == Some(chat.id.as_str());
        let status = partners(chat, &self.me)
            .next()
            .map(|user| user.status.clone())
            .unwrap_or_default();
        let preview = chat
            .last_message
            .as_ref()
            .map(|message| {
                if message.message_from == self.me {
                    format!("You: {}", message.message)
                } else {
                    message.message.clone()
                }
            })
            .unwrap_or_default();
        let unread = chat.unread_count > 0;
        let id = chat.id.clone();
        let ring = focus_ring(&theme);
        div()
            .id(gpui::SharedString::from(format!("room-{}", chat.id)))
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .h(px(58.))
            .mx(px(8.))
            .px(px(8.))
            .rounded(theme.metrics.radius)
            .cursor_pointer()
            .tab_index(0)
            .focus_visible(move |style| style.shadow(ring))
            .map(|row| {
                if selected {
                    row.bg(colors.row_selected)
                } else {
                    row.hover(|row| row.bg(colors.control_hover))
                }
            })
            .on_click(cx.listener(move |view, _, window, cx| {
                view.open(id.clone(), cx);
                view.focus(window, cx);
            }))
            .child(avatar(&chat.chat_name, status_color(&status, &theme), px(34.), cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(if unread {
                                        theme.type_scale.title_weight
                                    } else {
                                        theme.type_scale.label_weight
                                    })
                                    .text_color(colors.text)
                                    .child(chat.chat_name.clone()),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(theme.type_scale.caption)
                                    .font_features(tabular_figures())
                                    .text_color(colors.text_faint)
                                    .child(day_and_time(&chat.last_update)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme.type_scale.caption + px(1.))
                                    .text_color(if unread {
                                        colors.text
                                    } else {
                                        colors.text_muted
                                    })
                                    .child(preview),
                            )
                            .when(unread, |line| {
                                line.child(
                                    div()
                                        .flex_shrink_0()
                                        .px(px(6.))
                                        .h(px(17.))
                                        .flex()
                                        .items_center()
                                        .rounded(px(9.))
                                        .bg(colors.accent)
                                        .text_size(theme.type_scale.caption)
                                        .font_weight(theme.type_scale.label_weight)
                                        .font_features(tabular_figures())
                                        .text_color(colors.on_accent)
                                        .child(number(chat.unread_count)),
                                )
                            }),
                    ),
            )
    }

    fn room_list(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let needle = self.search.read(cx).value().trim().to_lowercase();
        let body = match &self.chats {
            Loaded::Loading => div().p(px(14.)).child(kit::note("Loading conversations.", cx)),
            Loaded::Failed(reason) => div().flex_1().child(kit::failed(
                "Could not load the conversations",
                reason,
                cx.listener(|view, _, _, cx| view.load(cx)),
                cx,
            )),
            Loaded::Ready(chats) if chats.is_empty() => div().flex_1().child(placeholder(
                IconName::MessageCircle,
                "No conversations yet",
                "A warframe.market message from another trader starts one.",
                cx,
            )),
            Loaded::Ready(chats) => {
                let rooms: Vec<_> = chats
                    .iter()
                    .filter(|chat| {
                        needle.is_empty() || chat.chat_name.to_lowercase().contains(&needle)
                    })
                    .map(|chat| self.room(chat, cx))
                    .collect();
                if rooms.is_empty() {
                    div()
                        .p(px(14.))
                        .child(kit::note("No conversation matches the search.", cx))
                } else {
                    div().flex().flex_col().gap(px(2.)).children(rooms)
                }
            }
        };
        div()
            .w(px(LIST_WIDTH))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(6.))
                    .px(theme.metrics.gutter)
                    .pt(px(14.))
                    .pb(px(10.))
                    .child(search_field(&self.search, px(LIST_WIDTH - 66.), window, cx))
                    .child(
                        IconButton::new(
                            "chat-refresh",
                            IconName::RefreshCw,
                            "Reload the conversations from warframe.market",
                        )
                        .on_click(cx.listener(|view, _, window, cx| view.refresh(window, cx))),
                    ),
            )
            .child(
                div()
                    .id("rooms")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .pb(px(8.))
                    .overflow_y_scroll()
                    .child(body),
            )
    }

    fn bubble(&self, message: &ChatMessage, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let mine = message.message_from == self.me;
        let bubble = div()
            .max_w(px(520.))
            .px(px(11.))
            .py(px(7.))
            .rounded(px(14.))
            .line_height(px(19.))
            .child(message.message.clone());
        let time = div()
            .px(px(4.))
            .text_size(theme.type_scale.caption)
            .font_features(tabular_figures())
            .text_color(colors.text_faint)
            .child(day_and_time(&message.send_date));
        div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .map(|column| {
                if mine {
                    column
                        .items_end()
                        .child(bubble.bg(colors.accent).text_color(colors.on_accent))
                } else {
                    column
                        .items_start()
                        .child(bubble.bg(colors.raised).text_color(colors.text))
                }
            })
            .child(time)
    }

    fn conversation(&self, chat: &Chat, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let partner = partners(chat, &self.me).next();
        let status = partner.map(|user| user.status.clone()).unwrap_or_default();
        let detail = match partner {
            Some(user) => format!(
                "{}, reputation {}",
                status_label(&user.status),
                kit::decimal(user.reputation)
            ),
            None => String::new(),
        };
        let (id, name) = (chat.id.clone(), chat.chat_name.clone());
        let header = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .h(px(56.))
            .px(theme.metrics.gutter)
            .border_b_1()
            .border_color(colors.border)
            .child(avatar(&chat.chat_name, status_color(&status, &theme), px(32.), cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(kit::heading(chat.chat_name.clone(), cx))
                    .child(kit::note(detail, cx)),
            )
            .child(
                IconButton::new("chat-delete", IconName::Trash, "Delete this conversation")
                    .hover_tone(colors.negative)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.ask_delete(id.clone(), name.clone(), window, cx);
                    })),
            );

        let messages = match &self.messages {
            Loaded::Loading => div().p(px(16.)).child(kit::note("Loading messages.", cx)),
            Loaded::Failed(reason) => {
                let id = chat.id.clone();
                div().flex_1().child(kit::failed(
                    "Could not load the messages",
                    reason,
                    cx.listener(move |view, _, _, cx| view.open(id.clone(), cx)),
                    cx,
                ))
            }
            Loaded::Ready(messages) if messages.is_empty() => div().flex_1().child(placeholder(
                IconName::MessageSquare,
                "No messages yet",
                "Write the first one below.",
                cx,
            )),
            Loaded::Ready(messages) => div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .p(theme.metrics.gutter)
                .children(messages.iter().map(|message| self.bubble(message, cx))),
        };

        let length = self.draft(cx).chars().count();
        let over = length > MESSAGE_LIMIT;
        let composer = div()
            .flex()
            .flex_shrink_0()
            .flex_col()
            .gap(px(5.))
            .px(theme.metrics.gutter)
            .py(px(10.))
            .border_t_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().min_w_0().child(Input::new(&self.composer)))
                    .child(
                        text_button("chat-send", "Send", cx)
                            .primary()
                            .icon(Icon::new(IconName::Send).size(theme.metrics.icon - px(1.)))
                            .disabled(self.sending || length == 0 || over)
                            .on_click(cx.listener(|view, _, window, cx| view.send(window, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(theme.type_scale.caption)
                    .text_color(colors.text_faint)
                    .child("Enter sends the message.")
                    .child(
                        div()
                            .font_features(tabular_figures())
                            .when(over, |count| count.text_color(colors.negative))
                            .child(format!("{length} of {MESSAGE_LIMIT} characters")),
                    ),
            );

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("messages")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(messages),
            )
            .child(composer)
    }
}

/// A circle with the first letter of a name and a status dot.
fn avatar(name: &str, status: Hsla, size: gpui::Pixels, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let letter: String = name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    div()
        .relative()
        .flex_shrink_0()
        .size(size)
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(size)
                .bg(colors.control_active)
                .font_weight(theme.type_scale.title_weight)
                .text_color(colors.text)
                .child(letter),
        )
        .child(
            div()
                .absolute()
                .right(px(-1.))
                .bottom(px(-1.))
                .size(px(11.))
                .rounded(px(11.))
                .bg(colors.surface)
                .flex()
                .items_center()
                .justify_center()
                .child(dot(status, px(7.))),
        )
}

impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        if self.active.is_some() && std::mem::take(&mut self.send_sample) {
            self.composer.update(cx, |composer, cx| {
                composer.set_value("Deal, 98p works. Inviting you now.", window, cx);
            });
            self.send(window, cx);
        }
        let list = self.room_list(window, cx);
        let pane = match self.active_chat().cloned() {
            Some(chat) => self.conversation(&chat, cx),
            None => div().flex_1().h_full().child(placeholder(
                IconName::MessageCircle,
                "No conversation selected",
                "Pick a conversation on the left to read and answer it.",
                cx,
            )),
        };
        div()
            .size_full()
            .flex()
            .bg(theme.colors.surface)
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text)
            .child(list)
            .child(pane)
    }
}

//! Settings, user and login, the live scraper, logs and debug info, and the
//! in-game inventory.

use std::{future::Future, path::PathBuf, sync::Mutex};

use qf_app_lib::{
    State,
    app::{AppState, User as BackendUser},
    commands::{app, auth, debug, live_scraper, logs, user, warframe_gdpr, wf_inventory},
};
use serde_json::{Value, json};

use super::{dto, list, to_backend};
use crate::{
    data::{DataError, DataResult, DataSource, convert},
    model::*,
};

fn user_model(user: BackendUser) -> DataResult<User> {
    let app = State::<Mutex<AppState>>::get();
    let (order_limit, auctions_limit) = {
        let app = app
            .lock()
            .map_err(|_| DataError::new("DataSource:User", "App state lock poisoned"))?;
        (
            app.wfm_client.order().get_order_limit() as i64,
            app.wfm_client.auction().get_auction_limit() as i64,
        )
    };
    let banned = if user.wfm_banned {
        Some(Ban {
            service: BanService::WarframeMarket,
            until: user.wfm_banned_until,
            reason: user.wfm_banned_reason,
        })
    } else if user.qf_banned {
        Some(Ban {
            service: BanService::Quantframe,
            until: user.qf_banned_until,
            reason: user.qf_banned_reason,
        })
    } else {
        None
    };
    Ok(User {
        anonymous: user.anonymous,
        wfm_status: UserStatus::parse(&user.wfm_status),
        wfm_id: user.wfm_id,
        wfm_username: user.wfm_username,
        wfm_avatar: user.wfm_avatar,
        platform: user.platform,
        locale: user.locale,
        order_limit,
        auctions_limit,
        unread_messages: user.unread_messages,
        verification: user.verification,
        banned,
    })
}

async fn scraper_state() -> DataResult<LiveScraperState> {
    let state = live_scraper::live_scraper_get_state(State::get()).await?;
    Ok(LiveScraperState {
        is_running: state["is_running"].as_bool().unwrap_or(false),
        message: None,
    })
}

async fn set_scraper_running(running: bool) -> DataResult<LiveScraperState> {
    if scraper_state().await?.is_running != running {
        live_scraper::live_scraper_toggle(State::get()).await?;
    }
    scraper_state().await
}

fn ee_log_query(params: &EeLogParams) -> Value {
    let mut query = list(&ListParams {
        page: params.page,
        limit: params.limit,
        query: params.query.clone(),
        ..ListParams::default()
    });
    query.insert("hide_empty".into(), json!(params.hide_empty));
    query.into()
}

impl DataSource {
    pub fn app_get_app_info(&self) -> impl Future<Output = DataResult<AppInfo>> + Send + 'static {
        let fixture_mode = self.fixture_mode;
        self.call(async move {
            let info = app::app_get_app_info(State::get()).await?;
            let text = |key: &str| info[key].as_str().unwrap_or_default().to_owned();
            let flag = |key: &str| info[key].as_bool().unwrap_or(false);
            Ok(AppInfo {
                name: text("name"),
                version: text("version"),
                description: text("description"),
                authors: text("authors"),
                is_dev: flag("is_dev"),
                use_temp_db: flag("use_temp_db"),
                is_pre_release: flag("is_pre_release"),
                tos_uuid: text("tos_uuid"),
                fixture_mode,
            })
        })
    }

    pub fn app_get_settings(&self) -> impl Future<Output = DataResult<Settings>> + Send + 'static {
        self.call(async move { Ok(app::app_get_settings(State::get()).await?) })
    }

    pub fn app_get_default_settings(
        &self,
    ) -> impl Future<Output = DataResult<Settings>> + Send + 'static {
        self.call(async move { Ok(app::app_get_default_settings().await?) })
    }

    /// Saves the settings and applies them to the log parser and inventory
    /// source. Returns what the backend stored.
    pub fn app_update_settings(
        &self,
        settings: Settings,
    ) -> impl Future<Output = DataResult<Settings>> + Send + 'static {
        self.call(async move {
            Ok(
                app::app_update_settings(settings, State::get(), State::get(), State::get())
                    .await?,
            )
        })
    }

    /// The signed-in user (`auth_me`).
    pub fn user(&self) -> impl Future<Output = DataResult<User>> + Send + 'static {
        self.call(async move { user_model(auth::auth_me(State::get()).await?) })
    }

    /// Signs in to warframe.market and Quantframe. Fixture mode returns an error.
    pub fn auth_login(
        &self,
        email: String,
        password: String,
    ) -> impl Future<Output = DataResult<User>> + Send + 'static {
        self.call(async move {
            user_model(auth::auth_login(email, password, State::get(), State::get()).await?)
        })
    }

    pub fn auth_logout(&self) -> impl Future<Output = DataResult<User>> + Send + 'static {
        self.call(async move { user_model(auth::auth_logout(State::get(), State::get()).await?) })
    }

    /// Sets your warframe.market status over the websocket. Fixture mode
    /// returns an error.
    pub fn user_set_status(
        &self,
        status: UserStatus,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move {
            Ok(user::user_set_status(status.as_str().to_owned(), State::get()).await?)
        })
    }

    pub fn live_scraper_get_state(
        &self,
    ) -> impl Future<Output = DataResult<LiveScraperState>> + Send + 'static {
        self.call(scraper_state())
    }

    /// Starts the live scraper. In fixture mode it is marked running and
    /// makes no requests.
    pub fn live_scraper_start(
        &self,
    ) -> impl Future<Output = DataResult<LiveScraperState>> + Send + 'static {
        self.call(set_scraper_running(true))
    }

    pub fn live_scraper_stop(
        &self,
    ) -> impl Future<Output = DataResult<LiveScraperState>> + Send + 'static {
        self.call(set_scraper_running(false))
    }

    /// Writes a zip of the logs to the desktop and returns its path.
    pub fn log_export(&self) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        self.call(async move {
            let path = logs::log_export(State::get(), State::get(), State::get()).await?;
            Ok(PathBuf::from(path))
        })
    }

    pub fn debug_get_wfm_state(
        &self,
    ) -> impl Future<Output = DataResult<WfmDebugState>> + Send + 'static {
        self.call(async move {
            let state = debug::debug_get_wfm_state(State::get())?;
            Ok(convert::wfm_debug_state(&state))
        })
    }

    pub fn debug_get_ee_logs(
        &self,
        params: &EeLogParams,
    ) -> impl Future<Output = DataResult<Paginated<EeLogLine>>> + Send + 'static {
        let query = ee_log_query(params);
        self.call(async move {
            let page = debug::debug_get_ee_logs(dto(query)?, State::get())?;
            Ok(convert::paginated(page, |line| EeLogLine {
                index: line.index,
                line: line.line,
                date: line.date,
            }))
        })
    }

    pub fn debug_export_ee_logs(
        &self,
        params: &EeLogParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = ee_log_query(params);
        self.export(path, async move {
            debug::debug_export_ee_logs(dto(query).map_err(to_backend)?, State::get()).await
        })
    }

    /// Whether a Warframe GDPR export has been loaded into the log parser.
    pub fn wfgdpr_get_state(&self) -> impl Future<Output = DataResult<GdprState>> + Send + 'static {
        self.call(async move {
            let state = warframe_gdpr::wfgdpr_get_state(State::get()).await?;
            Ok(GdprState {
                was_initialized: state["was_initialized"].as_bool().unwrap_or(false),
            })
        })
    }

    pub fn wf_inventory_get_rivens(
        &self,
        params: &ListParams,
    ) -> impl Future<Output = DataResult<Paginated<InventoryRiven>>> + Send + 'static {
        let query = list(params);
        self.call(async move {
            let value =
                wf_inventory::wf_inventory_get_rivens(dto(query.into())?, State::get()).await?;
            Ok(convert::inventory_rivens(&value))
        })
    }

    pub fn wf_inventory_get_syndicates(
        &self,
        params: &ListParams,
    ) -> impl Future<Output = DataResult<Paginated<InventorySyndicate>>> + Send + 'static {
        let query = list(params);
        self.call(async move {
            let value =
                wf_inventory::wf_inventory_get_syndicates(dto(query.into())?, State::get()).await?;
            Ok(convert::inventory_syndicates(&value))
        })
    }

    /// Reads the inventory again from its source.
    pub fn wf_inventory_update(&self) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(wf_inventory::wf_inventory_update(State::get()).await?) })
    }
}

use serde_json::{Value, json};

use crate::kit;

fn now() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64);
    format!("{}Z", kit::utc_date_time(millis).replace(' ', "T"))
}

pub fn payload(kind: &str) -> Option<Value> {
    match kind {
        "sale" => Some(json!({
            "playerName": "Vexi_Nox",
            "tradeTime": now(),
            "type": "sale",
            "platinum": 320,
            "credits": 0,
            "items": [
                {
                    "raw": "Mesa Prime Set",
                    "quantity": 2,
                    "unique_name": "/Lotus/Powersuits/Cowgirl/MesaPrime",
                    "item_type": "Set",
                    "properties": {
                        "name": "Mesa Prime Set",
                        "wfm_url": "mesa_prime_set",
                        "price": 67,
                        "isWishListItem": false
                    }
                },
                {
                    "raw": "Primed Flow",
                    "quantity": 1,
                    "unique_name": "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarPowerMaxModExpert",
                    "sub_type": { "rank": 10 },
                    "item_type": "Mod",
                    "properties": {
                        "name": "Primed Flow",
                        "wfm_url": "primed_flow",
                        "price": 91,
                        "isWishListItem": false,
                        "subTypes": { "maxRank": 10 }
                    }
                },
                {
                    "raw": "Rubico Prime Set",
                    "quantity": 1,
                    "unique_name": "/Lotus/Weapons/Tenno/LongGuns/PrimeRubico/RubicoPrime",
                    "item_type": "Set",
                    "properties": {
                        "name": "Rubico Prime Set",
                        "wfm_url": "rubico_prime_set",
                        "price": 108,
                        "isWishListItem": false
                    }
                }
            ]
        })),
        "purchase" => Some(json!({
            "playerName": "Kiyomi_Tenno",
            "tradeTime": now(),
            "type": "purchase",
            "platinum": 250,
            "credits": 0,
            "items": [
                {
                    "raw": "Harrow Prime Set",
                    "quantity": 1,
                    "unique_name": "/Lotus/Powersuits/Priest/HarrowPrime",
                    "item_type": "Set",
                    "properties": {
                        "name": "Harrow Prime Set",
                        "wfm_url": "harrow_prime_set",
                        "price": 104,
                        "isWishListItem": true
                    }
                },
                {
                    "raw": "Arcane Fury",
                    "quantity": 2,
                    "unique_name": "/Lotus/Upgrades/CosmeticEnhancers/Offensive/LongMeleeDamageOnCrit",
                    "sub_type": { "rank": 0 },
                    "item_type": "Arcane",
                    "properties": {
                        "name": "Arcane Fury",
                        "wfm_url": "arcane_fury",
                        "price": 40,
                        "isWishListItem": true,
                        "subTypes": { "maxRank": 5 }
                    }
                },
                {
                    "raw": "Ayatan Anasa Sculpture",
                    "quantity": 1,
                    "unique_name": "/Lotus/Types/Items/FusionTreasures/OroFusexOrnamentA",
                    "sub_type": { "amber_stars": 2, "cyan_stars": 0 },
                    "item_type": "Misc",
                    "properties": {
                        "name": "Ayatan Anasa Sculpture",
                        "wfm_url": "ayatan_anasa_sculpture",
                        "price": 0,
                        "isWishListItem": false,
                        "requireSubType": true,
                        "subTypes": { "amberStars": 2, "cyanStars": 2 }
                    }
                }
            ]
        })),
        _ => None,
    }
}

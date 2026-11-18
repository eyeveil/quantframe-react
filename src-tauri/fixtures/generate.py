"""Builds the fixture files in this directory.

Inputs are public, unauthenticated warframe.market responses recorded once:
  curl -H 'Language: en' https://api.warframe.market/v2/items            > items.json
  curl -H 'Language: en' https://api.warframe.market/v2/riven/weapons    > riven_weapons.json
  curl -H 'Language: en' https://api.warframe.market/v2/riven/attributes > riven_attributes.json

Usage: python3 -I generate.py <dir with the three recorded files>
"""

import json
import random
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

OUT = Path(__file__).resolve().parent
SRC = Path(sys.argv[1])
NOW = datetime(2026, 11, 17, 18, 0, tzinfo=timezone.utc)
rng = random.Random(20261118)


def load(name):
    return json.loads((SRC / name).read_text())["data"]


def write(rel, data, indent=1):
    path = OUT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=indent, ensure_ascii=False) + "\n")


def ts(days_ago, hours=0):
    return (NOW - timedelta(days=days_ago, hours=hours)).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def hex_id():
    return "".join(rng.choice("0123456789abcdef") for _ in range(24))


items = load("items.json")
weapons = load("riven_weapons.json")
attributes = load("riven_attributes.json")
by_slug = {i["slug"]: i for i in items}

tradable = []
for i in sorted(items, key=lambda i: i["slug"]):
    entry = {
        "name": i["i18n"]["en"]["name"],
        "uniqueName": i["gameRef"],
        "wfmId": i["id"],
        "wfmUrl": i["slug"],
        "tradeTax": 0,
        "masteryReq": 0,
        "tags": i["tags"],
        "icon": i["i18n"]["en"].get("icon", ""),
        "bulkTradable": i.get("bulkTradable", False),
    }
    sub = {}
    if "maxRank" in i:
        sub["maxRank"] = i["maxRank"]
    if "subtypes" in i:
        sub["variants"] = i["subtypes"]
    if "maxAmberStars" in i:
        sub["amberStars"] = i["maxAmberStars"]
    if "maxCyanStars" in i:
        sub["cyanStars"] = i["maxCyanStars"]
    if sub:
        entry["subTypes"] = sub
    tradable.append(entry)
write("cache/items/TradableItems.json", tradable, indent=None)

write(
    "cache/items/Attributes.json",
    [
        {
            "uniqueName": a["gameRef"],
            "wfmId": a["id"],
            "wfmUrl": a["slug"],
            "exclusiveTo": a.get("exclusiveTo"),
            "formattedValue": "",
            "group": a.get("group", "default"),
            "highlightedLabel": a["i18n"]["en"]["name"],
            "label": a["i18n"]["en"]["name"],
            "unit": a.get("unit"),
            "positiveIsNegative": a.get("positiveIsNegative"),
            "prefix": a.get("prefix", ""),
            "suffix": a.get("suffix", ""),
        }
        for a in attributes
    ],
)
write("cache/lang/en.json", {})

STOCK_SLUGS = [
    "saryn_prime_set", "rhino_prime_set", "mesa_prime_set", "nova_prime_set", "volt_prime_set",
    "wukong_prime_set", "khora_prime_set", "gauss_prime_set", "rubico_prime_set", "soma_prime_set",
    "nikana_prime_set", "glaive_prime_set", "kronen_prime_set", "acceltra_prime_set", "tigris_prime_set",
    "primed_continuity", "primed_flow", "primed_pressure_point", "primed_reach", "primed_target_cracker",
    "blind_rage", "narrow_minded", "transient_fortitude", "overextended", "condition_overload",
    "galvanized_chamber", "galvanized_aptitude", "galvanized_scope", "arcane_energize", "arcane_grace",
    "arcane_avenger", "molt_augmented", "cascadia_flare", "primary_merciless", "secondary_deadhead",
    "ayatan_anasa_sculpture", "ayatan_zambuka_sculpture", "lith_g1_relic", "meso_n16_relic", "axi_l4_relic",
]
stock_slugs = [s for s in STOCK_SLUGS if s in by_slug]
missing = sorted(set(STOCK_SLUGS) - set(stock_slugs))
if missing:
    sys.exit(f"missing slugs in items.json: {missing}")

STATUSES = ["live", "live", "live", "pending", "to_low_profit", "no_buyers", "inactive", "overpriced"]


def sub_type_for(item, maxed):
    if "maxRank" in item:
        return {"rank": item["maxRank"] if maxed else 0}
    if "maxAmberStars" in item:
        return {"amber_stars": item["maxAmberStars"], "cyan_stars": item.get("maxCyanStars", 0)}
    if "subtypes" in item:
        return {"variant": item["subtypes"][0]}
    return None


def price_history(base, n):
    return [{"created_at": ts(n - k), "price": max(1, base + rng.randint(-8, 8))} for k in range(n)]


def common(item):
    return {
        "wfm_id": item["id"],
        "wfm_url": item["slug"],
        "item_name": item["i18n"]["en"]["name"],
        "item_unique_name": item["gameRef"],
    }


stock_items = []
for n, slug in enumerate(stock_slugs):
    item = by_slug[slug]
    bought = rng.randint(8, 140)
    row = common(item) | {
        "bought": bought,
        "list_price": bought + rng.randint(4, 45) if n % 5 else None,
        "owned": rng.randint(1, 6),
        "is_hidden": n % 13 == 12,
        "status": STATUSES[n % len(STATUSES)],
        "price_history": price_history(bought + 15, rng.randint(0, 6)),
        "created_at": ts(60 - n),
        "updated_at": ts(30 - n // 2),
        "changes": [],
    }
    sub = sub_type_for(item, n % 2 == 0)
    if sub:
        row["sub_type"] = sub
    stock_items.append(row)

WISH_SLUGS = ["harrow_prime_set", "baruuk_prime_set", "titania_prime_set", "arcane_fury",
              "garuda_prime_set", "primed_chamber", "wisp_prime_set", "arcane_velocity",
              "lato_vandal_set", "galvanized_crosshairs", "sevagoth_prime_set"]
wish_list = []
for n, slug in enumerate(WISH_SLUGS):
    item = by_slug[slug]
    price = rng.randint(15, 120)
    row = common(item) | {
        "quantity": rng.randint(1, 3),
        "list_price": price,
        "price_history": price_history(price, rng.randint(0, 4)),
        "status": ["live", "pending", "inactive"][n % 3],
        "created_at": ts(40 - n),
        "updated_at": ts(10 - n // 2),
        "is_hidden": False,
        "changes": [],
    }
    sub = sub_type_for(item, False)
    if sub:
        row["sub_type"] = sub
    wish_list.append(row)

RIVEN_WEAPONS = ["rubico", "kuva_bramma", "acceltra", "torid", "nikana", "glaive", "kronen",
                 "phenmor", "laetum", "tigris", "lex", "stropha", "gram", "magistar", "trumna"]
weapon_by_slug = {w["slug"]: w for w in weapons}
pos_attrs = [a for a in attributes if not a.get("negativeOnly")]
neg_attrs = [a for a in attributes if not a.get("positiveOnly")]
POLARITIES = ["madurai", "vazarin", "naramon"]
PREFIXES = ["Crita", "Toxi", "Acri", "Visi", "Hexa", "Lexi", "Sati", "Igni"]
SUFFIXES = ["cron", "tox", "tis", "ata", "dex", "nok", "can", "pha"]


def riven_attrs(group):
    def fits(a):
        return not a.get("exclusiveTo") or group in a["exclusiveTo"]

    chosen = rng.sample([a for a in pos_attrs if fits(a)], rng.choice([2, 3]))
    attrs = [
        {"wfmUrl": a["slug"], "positive": True, "value": round(rng.uniform(40, 180), 1),
         "formattedValue": ""}
        for a in chosen
    ]
    if rng.random() < 0.6:
        neg = rng.choice([a for a in neg_attrs if fits(a) and a not in chosen])
        attrs.append({"wfmUrl": neg["slug"], "positive": False, "value": -round(rng.uniform(10, 60), 1),
                      "formattedValue": ""})
    return attrs


stock_rivens = []
for n, slug in enumerate(RIVEN_WEAPONS):
    w = weapon_by_slug[slug]
    bought = rng.randint(20, 400)
    stock_rivens.append({
        "wfm_weapon_id": w["id"],
        "wfm_weapon_url": w["slug"],
        "weapon_name": w["i18n"]["en"]["name"],
        "weapon_type": w.get("rivenType", "rifle"),
        "weapon_unique_name": w["gameRef"],
        "sub_type": {"rank": 8 if n % 3 else 0},
        "mod_name": rng.choice(PREFIXES) + "-" + rng.choice(PREFIXES).lower() + rng.choice(SUFFIXES),
        "attributes": riven_attrs(w.get("rivenType", "rifle")),
        "mastery_rank": rng.randint(8, 16),
        "re_rolls": rng.randint(0, 40),
        "polarity": POLARITIES[n % 3],
        "bought": bought,
        "list_price": bought + rng.randint(20, 300) if n % 4 else None,
        "filter": {"enabled": False},
        "is_hidden": False,
        "comment": "",
        "status": STATUSES[n % len(STATUSES)],
        "uuid": "",
        "price_history": price_history(bought + 50, rng.randint(0, 4)),
        "created_at": ts(50 - n),
        "updated_at": ts(20 - n),
    })

TRADERS = ["Kiyomi_Tenno", "VoidRelay77", "CephalonGrey", "ArbiterOfHex", "LotusEater",
           "NightwaveFan", "PlatinumPilgrim", "TennoTrader42", "OrokinVault", "DuviriDrifter"]

transactions = []
for n in range(30):
    if n % 5 == 4:
        w = weapon_by_slug[RIVEN_WEAPONS[n % len(RIVEN_WEAPONS)]]
        base = {
            "wfm_id": w["id"], "wfm_url": w["slug"], "item_name": w["i18n"]["en"]["name"],
            "item_type": "riven", "item_unique_name": w["gameRef"], "sub_type": {"rank": 8},
            "tags": "riven," + w.get("rivenType", "rifle"),
        }
        price = rng.randint(60, 900)
    else:
        item = by_slug[stock_slugs[(n * 7) % len(stock_slugs)]]
        base = common(item) | {"item_type": "item", "tags": ",".join(item["tags"])}
        sub = sub_type_for(item, True)
        if sub:
            base["sub_type"] = sub
        price = rng.randint(8, 160)
    sale = n % 3 != 0
    transactions.append(base | {
        "transaction_type": "sale" if sale else "purchase",
        "quantity": 1 if base["item_type"] == "riven" else rng.randint(1, 3),
        "user_name": rng.choice(TRADERS),
        "price": price,
        "profit": rng.randint(-5, 60) if sale else None,
        "credits": rng.choice([0, 2000, 8000, 12000]),
        "created_at": ts(45 - n, rng.randint(0, 20)),
        "updated_at": ts(45 - n),
        "properties": None,
    })

trade_entries = []
for n in range(12):
    item = by_slug[stock_slugs[(n * 3 + 1) % len(stock_slugs)]]
    entry = {
        "wfm_id": item["id"],
        "name": item["i18n"]["en"]["name"],
        "price": rng.randint(5, 150),
        "group": ["item", "set", "mod"][n % 3],
        "tags": ",".join(item["tags"]),
        "created_at": ts(25 - n),
        "updated_at": ts(25 - n),
        "properties": None,
    }
    sub = sub_type_for(item, False)
    if sub:
        entry["sub_type"] = sub
    trade_entries.append(entry)

SYNDICATE_ITEMS = [
    ("scattered_justice", "Steel Meridian", "SteelMeridianSyndicate", "live", 18),
    ("justice_blades", "Steel Meridian", "SteelMeridianSyndicate", "pending", None),
    ("neutralizing_justice", "Steel Meridian", "SteelMeridianSyndicate", "to_low_profit", None),
    ("gilded_truth", "Arbiters of Hexis", "ArbitersSyndicate", "live", 30),
    ("blade_of_truth", "Arbiters of Hexis", "ArbitersSyndicate", "no_buyers", None),
    ("winds_of_purity", "New Loka", "NewLokaSyndicate", "live", 25),
    ("bright_purity", "New Loka", "NewLokaSyndicate", "pending", None),
    ("sequence_burn", "Cephalon Suda", "CephalonSudaSyndicate", "live", 16),
]
syndicate_items = []
for n, (slug, syndicate, syndicate_unique, status, price) in enumerate(SYNDICATE_ITEMS):
    item = by_slug[slug]
    syndicate_items.append({
        "wfm_id": item["id"],
        "wfm_url": slug,
        "item_name": item["i18n"]["en"]["name"],
        "item_unique_name": item["gameRef"],
        "sub_type": {"rank": 0},
        "list_price": price,
        "syndicate_name": syndicate,
        "syndicate_unique_name": syndicate_unique,
        "standing_cost": 25000,
        "status": status,
        "price_history": [],
        "created_at": f"2026-11-{10 + n:02d}T18:00:00.000Z",
        "updated_at": f"2026-11-{10 + n:02d}T18:00:00.000Z",
        "changes": [],
    })

SYNDICATE_COLOURS = {
    "SteelMeridianSyndicate": ("#e3683a", "#3a1e14"),
    "ArbitersSyndicate": ("#d6d6d6", "#2b2b2b"),
    "NewLokaSyndicate": ("#7fb069", "#1f2b1a"),
    "CephalonSudaSyndicate": ("#4fa3d9", "#13263a"),
}
SYNDICATE_TITLES = [
    {"level": level, "name": name, "minStanding": low, "maxStanding": high}
    for level, name, low, high in [
        (0, "Neutral", -5000, 5000),
        (1, "Rank 1", 0, 22000),
        (2, "Rank 2", 22000, 44000),
        (3, "Rank 3", 44000, 70000),
        (4, "Rank 4", 70000, 99000),
        (5, "Rank 5", 99000, 132000),
    ]
]
syndicates = {}
for entry in syndicate_items:
    unique = entry["syndicate_unique_name"]
    icon, background = SYNDICATE_COLOURS[unique]
    syndicate = syndicates.setdefault(unique, {
        "uniqueName": unique,
        "name": entry["syndicate_name"],
        "canSelect": True,
        "iconColour": icon,
        "backgroundColour": background,
        "titles": SYNDICATE_TITLES,
        "favours": [],
    })
    syndicate["favours"].append({"standingCost": 25000, "creditsCost": 0, "uniqueName": entry["item_unique_name"]})
syndicates["CephalonSudaSyndicate"]["favours"].append(
    {"standingCost": 25000, "creditsCost": 0, "uniqueName": by_slug["toxic_sequence"]["gameRef"]}
)
write("cache/items/Syndicate.json", list(syndicates.values()))

write("database.json", {
    "stock_items": stock_items,
    "stock_rivens": stock_rivens,
    "wish_list": wish_list,
    "transactions": transactions,
    "trade_entries": trade_entries,
    "syndicate_items": syndicate_items,
})

order_pool = [i for i in items if any(t in i["tags"] for t in ("set", "mod", "arcane_enhancement"))]
rng.shuffle(order_pool)
orders = {"sell_orders": [], "buy_orders": []}
for n, item in enumerate(order_pool[:60]):
    kind = "sell" if n < 36 else "buy"
    order = {
        "id": hex_id(),
        "type": kind,
        "platinum": rng.randint(5, 220),
        "quantity": rng.randint(1, 5),
        "visible": n % 11 != 10,
        "itemId": item["id"],
        "createdAt": ts(20 - n // 4, n % 24),
        "updatedAt": ts(5 - n // 15, n % 24),
    }
    sub = sub_type_for(item, kind == "sell")
    if sub:
        if "rank" in sub:
            order["rank"] = sub["rank"]
        if "variant" in sub:
            order["subtype"] = sub["variant"]
    orders[f"{kind}_orders"].append(order)
write("wfm_orders.json", orders)

auctions = []
for n, riven in enumerate(stock_rivens[:5]):
    attrs = [{"url_name": a["wfmUrl"], "positive": a["positive"], "value": a["value"]}
             for a in riven["attributes"]]
    price = (riven["list_price"] or riven["bought"] + 100)
    auctions.append({
        "id": hex_id(),
        "minimal_reputation": 0,
        "winner": None,
        "private": False,
        "visible": n != 4,
        "note_raw": "",
        "note": "",
        "starting_price": price,
        "buyout_price": price,
        "is_direct_sell": True,
        "top_bid": None,
        "created": ts(15 - n),
        "updated": ts(3),
        "platform": "pc",
        "closed": False,
        "is_marked_for": None,
        "marked_operation_at": None,
        "item": {
            "type": "riven",
            "weapon_url_name": riven["wfm_weapon_url"],
            "name": riven["mod_name"].lower(),
            "attributes": attrs,
            "re_rolls": riven["re_rolls"],
            "mastery_level": riven["mastery_rank"],
            "mod_rank": riven["sub_type"]["rank"],
            "polarity": riven["polarity"],
        },
        "name": riven["weapon_name"] + " " + riven["mod_name"],
        "mod_name": riven["mod_name"],
    })
write("wfm_auctions.json", {"auctions": auctions})

ME = "64f0c0ffee0000000000f1a7"
LINES = [
    ("them", "WTB {item} for {price}p"),
    ("me", "Sure, inviting now"),
    ("them", "Can you do {lower}p?"),
    ("me", "{price}p is already below the market price, sorry"),
    ("them", "ok deal, inv me"),
    ("me", "Invited, thanks for the trade"),
]
chats = []
for n in range(4):
    other = TRADERS[n]
    other_id = hex_id()
    item = by_slug[stock_slugs[n * 5]]
    price = rng.randint(20, 120)
    chat_id = hex_id()
    messages = []
    for k, (who, text) in enumerate(LINES[: 3 + n]):
        msg = text.format(item=item["i18n"]["en"]["name"], price=price, lower=price - 10)
        messages.append({
            "id": hex_id(),
            "chat_id": chat_id,
            "message": msg,
            "raw_message": msg,
            "send_date": ts(n, 6 - k),
            "message_from": ME if who == "me" else other_id,
        })
    chats.append({
        "chat_with": [{"id": other_id, "ingame_name": other, "reputation": float(rng.randint(0, 300)),
                       "status": ["online", "in_game", "offline", "online"][n]}],
        "unread_count": [2, 0, 1, 0][n],
        "last_update": messages[-1]["send_date"],
        "messages": messages,
        "id": chat_id,
        "chat_name": other,
    })
write("wfm_chats.json", {"chats": chats})

write("user.json", {
    "anonymous": False,
    "verification": True,
    "wfm_banned": False,
    "qf_banned": False,
    "wfm_id": ME,
    "wfm_token": "",
    "qf_token": "",
    "wfm_username": "FixtureTrader",
    "check_code": "",
    "locale": "en",
    "platform": "pc",
    "wfm_status": "invisible",
    "unread_messages": 3,
})
print({k: len(v) for k, v in {
    "tradable": tradable, "attributes": attributes, "stock_items": stock_items,
    "stock_rivens": stock_rivens, "wish_list": wish_list, "transactions": transactions,
    "trade_entries": trade_entries, "sell_orders": orders["sell_orders"],
    "buy_orders": orders["buy_orders"], "auctions": auctions, "chats": chats}.items()})

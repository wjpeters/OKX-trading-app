# Lokale API v0.4.0

Basis http://127.0.0.1:4780. Host-allowlist; geen CORS. Decimals als JSON-strings. UTC-timestamps in milliseconden, duurmetingen monotone klok. Leesroutes zijn GET; mutaties vereisen de hieronder beschreven lokale boundary en, indien ingesteld, een geldige sessie. Onbekende `/api/*` routes geven 404. POST op bestaande GET-routes geeft 405.

| Route | Contract |
|---|---|
| GET /api/health | Proces leeft; zegt niets over bruikbare marktdata |
| GET /api/ready | 200 wanneer DB/catalogus/publicfeed en paperworker gereed zijn; anders 503 |
| GET /api/status | Versie, PAPER_LOCAL, echte risk-/workerstatus, UNVERIFIED accountregio, verbindingen, gevalideerde berichtenteller en beperkte samplelatency |
| GET /api/market | `market` plus `server_time_ms`; instruments, tickers, books, trades, source freshness |
| GET /api/candles?instrument=BTC-EUR&bar=15m | candles, indicators, gaps, fresh, received_at_ms, REST- en rekentijden |
| GET /api/candles?...&after=<oude-timestamp-ms> | Oudere pagina via native history-candles, maximaal 300; historical=true, nooit fresh |
| WS /ws | Lokale markt-snapshots elke 250 ms; maximaal acht clients, geldige Origin verplicht |

Timeframes: 1m, 5m, 15m, 1H, 4H, 1Dutc. Instrument moet in werkelijk geladen watchlist staan. Input-/provider-/DB-fouten bij candleaanvraag leveren 503 met concrete error; control-routes gebruiken 403/413/422/429. Geen stale databasecache als stille fallback. Eerder ontvangen data in de browser wordt zichtbaar als fout/verouderd getoond.

Candleprijzen en volumes zijn strings, closed komt van OKX confirm. Indicatorpunten bevatten null tijdens warm-up. SMA20; EMA20 SMA-seed; Wilder RSI14 (vlak=50), MACD12/26/9, Wilder ATR14, Bollinger20 ±2 populatiestandaarddeviaties, UTC-typical-price VWAP, ruwe Stochastic %K14 (vlak=50), Wilder ADX14, relatieve OBV. VWAP vanaf geladen intraday historie kan onvolledig zijn; OBV heeft relatieve startnul. Geen tradingbeslissing gebruikt deze chartwaarden.

Voorbeelden:
```sh
rtk proxy curl --fail http://127.0.0.1:4780/api/status
rtk proxy curl --fail 'http://127.0.0.1:4780/api/candles?instrument=BTC-EUR&bar=15m'
```

## Fase 2 · PAPER_LOCAL

GET `/api/paper` geeft bevestigde virtuele state of `account:null`, setupvoorstellen, aannames en actuele workerstatus. GET `/api/replay/reports` geeft maximaal twintig opgeslagen replayrapporten. Geld wordt als decimalstring verzonden.

Alle POST-routes vereisen exact dezelfde Origin als Host, JSON, header `X-Terminal-Request: paper-v2`, maximaal 16 KiB, vier gelijktijdige opdrachten en zestig opdrachten/minuut. Strict schemas; onbekende velden zijn fouten. Geen live/demo-route.

| Route | Inhoud / betekenis |
|---|---|
| `/api/paper/setup` | `{account: Limits, manual: Limits, confirmed:true}`; eenmalig, geen reset |
| `/api/paper/preview` | Intent met client_id, bot_id, instrument, side BUY/SELL, kind MARKET/LIMIT, quantity, limit, stop, target, position_id; geeft preview_id en prijs/reserve |
| `/api/paper/submit` | `{preview_id, confirmed:true}`; hercontrole, atomair reserveren, duurzaam PENDING_SUBMIT; herhaling idempotent |
| `/api/paper/cancel` | `{order_id}`; PendingCancel houdt reserve tot lokale bevestiging |
| `/api/paper/control` | `{action, confirmed:true}`; PAUSE_ENTRIES, CANCEL_ENTRY_ORDERS, RESUME_ENTRIES of EMERGENCY_FLATTEN |
| `/api/paper/bots` | id, name, instrument, limits, fast, slow, order_eur, stop_bps, target_bps; immutable sma-cross-v1, start gepauzeerd |
| `/api/paper/bots/control` | `{bot_id, running}`; start/pauze, centrale gate verplicht |
| `/api/replay` | instrument, bar, fast, slow, capital, order_eur, stop_bps, target_bps, spread_bps, slippage_bps; onafhankelijke candle-replay |

Limits: capital, trade_risk, daily_loss, total_loss, drawdown, position, positions, orders, leverage, spread_bps, slippage_bps, data_age_ms, orders_per_minute, loss_streak, ai_budget. Geld en bp zijn decimalstrings. Aantallen en milliseconden zijn gehele JSON-getallen. Leverage is "1", AI-budget als afzonderlijke USD-cap, standaard "0". Fase 2 ondersteunt EUR-spotuitvoering. Zie PHASE2.md voor exacte definities.

Een fout geeft een zichtbare reden en HTTP 422 (validatie/risico/afhankelijke opslag), 403 (lokale boundary), 413 (body), 429 (capaciteit) of 404 (onbekende route). Een transportfout na submit betekent: controleer orders of herhaal hetzelfde preview_id, nooit een willekeurig nieuw client_id als blind retry.

## Studio, onderzoek en operatie

| Methode en route | Contract |
| --- | --- |
| GET /api/studio/documents?kind=pine | Laatste100 immutable versies van pine/graph/workspace/drawings |
| POST /api/studio/documents | `{kind,name,base_version,payload}`; optimistische versiecontrole, maximaal14KB payload |
| POST /api/studio/pine | `{source,instrument,bar,inputs?}`; ok,inputs,plots,intents,diagnostics,runtime,duration_ms; runtimefout geeft ok:false zonder voorstellen |
| POST /api/studio/graph/validate | Canonieke schema1-grafiek; topologische validatie en serverparametercontrole |
| POST /api/studio/deploy | `{bot_id,name,version}`; alleen bestaande gepauzeerde bot zonder exposure/orders |
| GET/POST /api/studio/alerts | GET regels/inbox; POST `{name,instrument,bar,field,comparison,value,trigger,cooldown_secs,bot_id?,drawing?}` |
| POST /api/studio/alerts/control | `{id,enabled}`; maximaal40 actief, ook bij hervatten |
| POST /api/studio/deliveries/read | Inbox als gelezen markeren |
| POST /api/studio/indicators | `{instrument,bar,settings}`; MA/oscillator/MACD/BB-inputs, native berekening |
| GET/POST /api/history | Manifest / import `{instrument,bar,pages,resume?}`; 1–10 pagina’s, standaard hervatten |
| GET /api/history/export?id=… | Het gevalideerde lokale Parquetbestand, geen vrij bestandspad |
| POST /api/replay/evaluate | Replayconfig → WALK_FORWARD met twee aparte OOS-vensters en gevoeligheid |
| POST /api/replay/pine | `{source,inputs?,config:ReplayConfig}` → PINE_REPLAY via dezelfde geïsoleerde runtime en riskkern |
| GET /api/replay/reports?mode=… | Laatste20 per REPLAY(default), PINE_REPLAY of WALK_FORWARD |
| GET /api/research | Bronnen, headlines, FNG, echte AI-kosten/reserveringen en envelopes |
| POST /api/jev/config | `{enabled,budget_usd,confirmed}`; zie UI voor ingevuld schema en caps |
| POST /api/jev/propose | `{instrument,bar}`; onderzoeksvoorstel, geen order |
| POST /api/research/compare | `{with_jev,without_jev}` bot-id’s; uitsluitend vergelijkbare werkelijke forward-deployments |
| GET /api/operations | Opslag, bescherming, authstatus, procesisolatie en vrijgavevoorwaarden |
| GET /api/metrics | Prometheus-tekst voor verbinding, workerleeftijd en reconnects |
| GET /api/auth/status | Of lokale auth is ingesteld en huidige sessie geldig is |
| POST /api/auth/login | `{password}`; gehashte sessie, HttpOnly/SameSiteStrict8h-cookie; CSRF/rategrenzen gelden ook hier |
| POST /api/auth/logout | Sessierecord intrekken en cookie wissen |

Alertfields: price,sma,ema,rsi,macd,atr,bb_upper,bb_lower,vwap,stochastic,adx,obv,risk_paused,bot_running,bot_drawdown,bot_total_loss,drawing_line. comparison=above/below; trigger=close/intrabar; cooldown10–86400s. Indicatoren vereisen close. drawing_line krijgt `{first_ms,second_ms,first_price,second_price}` en value0; vergelijking gebruikt koers minus geprojecteerde lijnprijs. Bekende positieve ankers/tijden vereist, geen schermcoördinaten.

Authpoort blijft lokaal HTTP; cookies zijn daarom niet als remote TLS-config te beschouwen. Health/readiness/authstatus zijn bereikbaar zonder login; overige API/WS vereisen een geldige sessie als auth is ingesteld. Verlopen WS-sessie sluit uiterlijk bij de30s-hercontrole. Geen `/api/live/orders`, private credentials, generieke providerproxy of Jev-executeroute.

# Acceptatie tegen de volledige bouwprompt

Willem heeft gevraagd alle verdere fasen te bouwen. Dit document beschrijft het werkelijk behaalde bewijs; het is **geen verklaring dat de volledige opdracht of fase5 is afgerond**. Account-, instrument-, budget- en limietkeuzes blijven voor Willem. Jev heeft nooit uitvoeringsautoriteit.

## Stand per fase

| Fase | Stand |
| --- | --- |
| 0 | Architectuur, threat model, providers/licenties, scope en capabilities; persoonlijke accountrechten nog onbekend |
| 1 | Echte publieke terminal, charts/indicatoren/status; uitgebreid met sequenceboek en multi-chart |
| 2 | Centrale risk/OMS, lokale paperadapter, serverbot en replay; herhaalde37 integratiechecks |
| 3 | Pine closed-bar, visuele Bot Studio, tekeningen en serveralerts gebouwd/getest; geavanceerde Pine/docking volgen nog |
| 4 | Jev Decisions-adapter/kosten/fixtures en echte nieuws/FNG-adapters, OOS en forwardvergelijkingsrapport gebouwd; echte Jev-call/vergelijking nog niet uitgevoerd |
| 5 | Securityhardening/auth/backuprestore en read-only Demo-onboarding gebouwd; operationele exchangegateway/reconciliatie/bescherming en accountacceptatie ontbreken |

## Bewijs en foutmatrix

94 Rust-tests + 11 Pine-tests; 87 geïsoleerde HTTP/integratiecontroles (37+26+14+10). Geen gebruikersrekening of echte orders. Bronbewijs staat in STATUS.md/evidence. Scheid fixture van realistische adapterproef:

| Eigenschap | Werkelijk getest | Nog open |
| --- | --- | --- |
| Book-data | Snapshot/delta seq/prev, heartbeat/reset/gap, echte publieke stream | Bulk tick/L2-dekking |
| Oude/gappende candles | Normalisatie, warm-up, gesloten bars, next-bar replay | Business-WS/intrabar/securityprofielen |
| Risk/gelijktijdig budget | Decimal/rekeningrowlock, één winnaar, bot-/accountlimieten | Echte fees/cashflows/conversies/subaccounts |
| OMS/partial/dedup | Lokale ACK/fill apart, reserves, crash/DB-uitval, verliezen na herstart | Exchangeacceptatie gevolgd door timeout en echte externe orders |
| Twee executors | Tweede lokale paperworker geweigerd | Live-credential/egress-fencing tussen hosts |
| Pine | Foutcode geeft geen intents, toekomstige data/ongeldige TA/inputs geweigerd, geïsoleerde processen | Breed adversarial corpus en verdere taalprofielen |
| Jev | Responseschema, state/TTL/confidence/kostenregels in fixtures; geen key geeft weigering | Echte providerresponse/timeout/late-response-kostenacceptatie en forwardvergelijking |
| Alerts | Serverprijs/lijn/bot/inboxdedup; zonder browser | Externe notificatiedienst niet gevraagd/ingesteld |
| Storage/auth | Drempel/fail-closed unit, AES-GCM-scope/tamper, DB+Parquet-restore, auth/expiry/logout/CSRF | Fysieke full-disk/hostdisaster en remote MFA/TLS |
| Performance | Native fixture, echte WS/HTTP, begrensde120-requestburst, Pinepreviewduur | Lange soak, volledige event→intent/submit→ack/fill-percentielen |
| Supply chain | Rust/npm en drie runtimeimages0 meldingen na hardening | Onafhankelijke review en onbekende kwetsbaarheden |

## Traceability

| Specsectie | Huidige invulling | Open grens |
| --- | --- | --- |
| 1–2 | Nieuw lokaal spotproject, Rust/React/PG/Parquet, execution-procesgrens | Geen cloud; echte gateway nog open |
| 3 | Werkruimtes, multichart, workspaces, chartlijnen/preview, commandpalette | Vrije docking, orderlijn-amendments, snelmodus |
| 4–5 | Native publieke EEA, sequence/gaps, historie/coverage, metadata/klok | Persoonlijke rechten/private reconcile, history-trades/business-WS |
| 6 | Tien instelbare indicatoren, negen tekentools en serveralerts | DeMARK, volumeprofile/orderflow/heatmaps |
| 7 | Echte parser/AST/Monaco/inputs/versies, geïsoleerd closed-bar-profiel | Verdere Pine-semantiek zoals expliciet in matrix |
| 8–9 | Canonieke graphcompiler, templates, serverdeployments, Jev-proposals/costguard | Echte Jev-acceptatie, later gecontroleerde livepromotie |
| 10–11 | Centrale paper-risk/OMS/stop/reserves/HWM/lease | Echte rekeningadministratie, exchange-native bescherming/fencing |
| 12 | Replay/Pine/walk-forward/OOS/gevoeligheid en forwardrapport | Demo/LIVE-acceptatie, tickboekreplay |
| 13 | OKX-news/status en dagelijkse FNG, bron/TTLstatus | CryptoPanic/CoinGecko plan/rechten |
| 14–15 | Echte beperkte metingen, localhost, encryptie/auth/scans/backuprestore/logrotatie | Langdurigeoperatie/amd64/remote beveiliging |
| 16 | Bron/locks/migraties/Docker/handleidingen/tests/status | Volledige exchangefoutmatrix en bewuste livevrijgave |

Concrete vervolgstappen en implementatiehiaten: [REMAINING.md](REMAINING.md). Die scope is niet stilzwijgend vervallen.

## Aanvullende UI-acceptatie · 4 oktober 2026

[De tekenbediening](DRAWING_TOOLS.md) is herzien en in de browser getoetst voor alle negen vormen, selectie/slepen, undo/redo, opslag, live verversing, thema's en multichart. Tien gerichte frontendtests en de productiebuild slagen. Voor het exacte bewijs en de testgrenzen zie de handleiding; dit verandert de open exchange-/fase5-acceptatie niet.

# Capabilities · 0.4.0 · 2026-10-02

Publieke EEA-data bewijst geen persoonlijke accountrechten. De app gebruikt maximaal twaalf werkelijk beschikbare EUR/USDC-spotparen. Paper-uitvoering is EUR spot1×. Geen USDT-/derivatenaanname, regiofallback of poort8443.

| Capability | Gebouwd en uitgevoerd | Grens |
| --- | --- | --- |
| public/time, public/instruments | Periodieke klok/metadata, tick/lot/min | Publieke toegang, geen accounttoestemming |
| market/candles, history-candles | REST/caching, resumable Parquet, coverage/export | 1–10 pagina’s/opdracht; geen onbeperkte historie |
| public WS tickers/trades/books | Echte streams; snapshot/delta seq/prev, heartbeat/reset/gap/resync | Geen VIP-kanalen; UI top5, candles nog REST |
| market/history-trades, business candles | Niet geïmplementeerd | Tick/L2-backfills en business-WS nog open |
| account/config, instruments, fees, balance, positions | Native signer en read-only Demo-onboarding gebouwd | Geen nieuwe Demo-key, dus nog geen accountacceptatie |
| orders-pending, fills, individuele orderstatus | Read-only transport gebouwd | Geen continue reconciliatie/volledige accountboekhouding |
| private WS orders/account/positions | Login + bounded subscriptionprobe gebouwd | Niet uitgevoerd zonder key; geen duurzame updateworker |
| trade/order | Transport met ontoegankelijke approvalconstructor | Geen operationele ordergateway of vrijgave |
| cancel/amend, order-algo, cancel-all-after | Endpointallowlist voorbereid | Geen operationele implementatie/accountacceptatie |
| PAPER_LOCAL | Vaste risk/OMS + lokale boekadapter + bots/replay | Expliciete simulatieaannames, virtueel geld |
| OKX_DEMO / LIVE | Strikt gescheiden onboardingproces; terminal weigert beide modi | Uitvoering ontbreekt; LIVE hard geblokkeerd |
| Pine/Bot Studio/serveralerts | Werkende closed-bar-runtime, graphplan en alerts | Verdere Pineprofielen/docking blijven open |
| Jev/OpenRouter | Decisions-schema, keyencryptie, kosten/TTL/breaker, graphfilter | Providerfixtures getest; echte call/forwardvergelijking nog niet |
| OKX-news / Alternative.me | Echte periodieke bronnen en bronstatus | Regionale nieuwsroute kan leeg zijn; FNG is dagelijkse BTC-index |
| CryptoPanic/CoinGecko/DeMARK | Zichtbaar beperkt | Plan/rechten/licentie niet bevestigd |

## Endpoints en conservatieve limiter

Alle OKX REST-paden onder `/api/v5`, vaste `https://eea.okx.com`; public WS `wss://wseea.okx.com/ws/v5/public`, Demo-private `wss://wseeapap.okx.com/ws/v5/private`, poort443. Demo voegt `x-simulated-trading:1` toe. Geen generieke proxy of withdrawalendpoint.

Gedocumenteerde publieke maxima bij controle: time10/2s, candles40/2s, history-candles20/2s per IP; WS connect3/s/IP,480 subscription/loginhandelingen/verbinding/uur. De app houdt minimaal150ms tussen publieke REST-starts, één request tegelijk; private onboarding250ms met5s-timeout. Deze conservatieve eigen limieten vervangen geen toekomstige per-account/instrumentorderquota. Beschermingscapaciteit moet in fase5 apart worden gereserveerd.

Echte onboarding moet regio, accounttype/subaccount, instrumenten, settlement, fees, ordertypes en native bescherming aantonen. De gebruikte fee-markt wordt uit de accounttoegankelijke EUR-spotlijst gekozen, niet uit een globale aanname. Credentials alleen met noodzakelijke rechten en zonder withdrawals. Zie PROVIDERS_LICENSES.md voor bronnen en REMAINING.md voor uitvoeringswerk.

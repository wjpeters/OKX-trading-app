# Tijdframes in de terminal

Uitbreiding van 4 oktober 2026. De koersgrafiek biedt veertien intervallen:

- Minuten: 1, 3, 5, 15 en 30 minuten.
- Uren: 1, 2, 4, 6 en 12 uur.
- Langere periodes: 1 dag, 1 week, 1 maand en 3 maanden (kwartaal).

De snelle knoppen bevatten 1m, 5m, 15m, 1H, 4H, 1D, **1W** en **1M**. Het menu **Alle tijdframes** bevat de volledige lijst, gegroepeerd per soort. Kleine `m` betekent minuten; hoofdletter `M` betekent maanden. De gekozen periode blijft in de bestaande browservoorkeur en werkruimte bewaard.

## Kalender en datakwaliteit

De grafiek gebruikt echte publieke OKX EEA-candles. Voor 6 uur en langer worden de UTC-varianten gebruikt. Weken openen maandag 00:00 UTC; maanden openen op de eerste; kwartalen openen op 1 januari, april, juli en oktober. De laatste candle kan nog wijzigen. De beschikbare historie hangt van instrument en noteringsdatum af; 300 candles is een maximum, geen garantie.

Maanden worden niet als 30 dagen behandeld. Normalisatie, ontbrekende-candlecontrole, actualiteit en indicatoren gebruiken de volgende kalendergrens. Een februaricandle veroorzaakt dus geen onterecht datagat of herstart van indicatoren. Ontbreekt een echte periode, dan reset de indicatoropbouw wel. Tekentijdpunten in de lege ruimte naast maand-/kwartaalcandles volgen eveneens de kalender. Maand-/kwartaallabels tonen maand en jaar.

Dag-VWAP kan niet worden herleid uit week-/maand-OHLCV en blijft op die intervallen leeg. Andere indicatoren blijven leeg zolang hun benodigde aantal candles ontbreekt.

Serveralerts ondersteunen de nieuwe kalendergrenzen. De aparte Pine- en replay-runtime gebruikt nog vaste tijdstappen: maand/kwartaal zijn daar niet toegevoegd en geven een expliciete melding. De replaykeuzelijst is buiten deze grafiekuitbreiding gebleven. Geen wijziging aan ordertoestemming, botstart, rekening of limieten.

Ontwikkelreferentie: [officiële OKX-candle-API](https://app.okx.com/docs-v5/en/#order-book-trading-market-data-get-candlesticks). Getoetst aan publieke antwoorden voor 1Wutc, 1Mutc en 3Mutc op 4 oktober 2026.

## Verificatie

- Kalenderregressies voor maandaggrenzen, maandlengte, schrikkeljaren, jaar-/kwartaalwisseling, deduplicatie en echte datagaten.
- Indicatorregressie: veertig maandcandles leveren doorlopende EMA/SMA/RSI; een echte ontbrekende maand reset de opbouw.
- Frontendtest voor maand- en kwartaalankers buiten de geladen historie, naast de bestaande tekentests.
- `scripts/timeframe_checks.py` toetst de lokale API met echte publieke data, alle veertien intervallen en historische week-/maandpaginatie; bewijs in `evidence/timeframes-live-2026-10-04.json` (geslaagd op 4 oktober 2026).
- De backendcontrole gebruikt de vastgezette Docker-omgeving. De lokaal geïnstalleerde Rust-versie is te oud voor de bestaande SQLx-dependency.

### Gecontroleerde lokale versie

- 100 Rust-tests (91 terminal + 9 onboarding), 11 frontendtekentests en TypeScript/productiebuild geslaagd. Backendbewijs: `evidence/timeframes-rust-tests-2026-10-04.txt`; runtimebuild: `evidence/timeframes-build-2026-10-04.txt`.
- Alle veertien BTC-EUR-tijdframes actueel en zonder gaten. Beschikbare historie op het testmoment: 150 weekcandles, 36 maandcandles, 13 kwartaalcandles. De kwartaalreeks is nog te kort voor de standaard EMA20/RSI14; die blijven terecht leeg.
- Historische week- en maandpaginatie geslaagd. De controles gebruiken alleen publieke marktdata.
- Browser: week, maand en kwartaal laden; maand blijft na herladen geselecteerd. Screenshot: `evidence/timeframes-ui-2026-10-04.jpg`.
- Runtime op poort 4780: running/healthy.

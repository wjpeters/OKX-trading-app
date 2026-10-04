# Fase 2: lokale risk/OMS, paper, bot en replay

Deze fase draait uitsluitend in `PAPER_LOCAL`. De publieke OKX-adapter bevat alleen vier toegestane GET-endpoints en publieke WebSockets. Er is geen signer, order-POST, private kanaal, demo- of liveadapter. `LIVE`, `OKX_DEMO` en `READ_ONLY` worden door deze versie vóór secretinlezing geweigerd. Jev staat uit; zijn voorstelvalidator is geen orderbevoegdheid.

## Gebruik

1. Open Paper en bevestig de virtuele EUR-rekening en het handmatige deelbudget. Alle bedragen zijn bewerkbare voorstellen, geen geadviseerde handelsbudgetten. De rekening ontstaat pas na bevestiging.
2. Het orderticket gebruikt echte EUR-spotmetadata. Vul hoeveelheid, eventueel limietprijs, verplichte stop en optioneel target in. De risicokern controleert alles voordat een preview verschijnt. De preview is 15 seconden geldig.
3. Bevestig de paperorder. Een duurzaam `PENDING_SUBMIT` is nog geen fill. Orders, fills, posities, limieten en de laatste honderd auditregels zijn apart zichtbaar. De volledige audit blijft in PostgreSQL.
4. Maak in Bot Studio een eenvoudige SMA-bot met eigen budget. Start hem bewust. Hij verwerkt gesloten 1-minuutcandles op de server, ook zonder browser. Pauzeren annuleert de instaprest en behoudt positiecontrole.
5. Backtests voert dezelfde SMA- en risicokern uit op maximaal 300 recente echte candles. Rapporten staan in een aparte tabel; de paperrekening verandert niet. Download het JSON-rapport voor alle fills, beslissingen, parameters en aannames.

## Vaste limieten en grondslagen

Verplicht voor rekening én iedere bot/handmatig deelbudget: kapitaal, risico per transactie, dagverlies, cumulatief verlies, drawdown, exposure per onderliggende munt, aantallen posities/orders, leverage, spread, slippage, ouderdom, orderfrequentie, verliesreeks en AI-budget. Geld gebruikt Rust Decimal en JSON-decimalstrings. Tick- en lotvereisten worden exact gecontroleerd. Orders worden niet stilzwijgend afgerond; de bot rondt sizing naar beneden en beschermingsprijzen naar de vereiste tick.

- EUR-spotparen uit de gecontroleerde watchlist. Geen FX-conversies, shorts of USDC-uitvoering in fase 2. Chartanalyse van USDC blijft mogelijk.
- Leverage exact 1 en AI-budget exact 0. De API weigert andere waarden. Funding is nul bij dit spotmodel.
- Botkapitaal wordt uit hetzelfde rekeningbudget toegewezen; de som mag het rekeningkapitaal niet overschrijden. Het is virtuele boekhouding, geen echte subaccountisolatie.
- Equity = cash + resterende inventaris × actuele biedprijs × (1 − geraamde exitfee). Entryfees zitten al in cash/kostprijs. Reserves beperken vrij besteedbaar geld maar worden niet opnieuw van equity afgetrokken.
- De enige cashflow is het expliciet bevestigde virtuele startsaldo. Er bestaan geen stortings-, opname-, reset- of limietverhogingsroutes. Hierdoor zijn cashflowcorrecties en funding in deze scope exact nul. Uitbreiding moet deze ledger eerst toevoegen.
- Daggrens is **00:00 UTC**, expliciet in de setup bevestigd. Bij de eerste verse waardering na de grens wordt de laatst bekende vorige equity als dagstart vastgelegd. Bij een onderbreking over middernacht telt de onwaargenomen daling conservatief mee op de eerst hervatte dag. Dit is geen achteraf gereconstrueerde exacte middernachtkoers.
- Dagverlies = max(0, dagstart − equity); cumulatief = max(0, startbaseline − equity); high-water-mark = max(vorige HWM, equity); drawdown = max(0, HWM − equity). Bedraglimieten staan vast; percentages worden bij hun expliciete actuele grondslag getoond.
- Zonder verse biedwaardering voor **alle** open posities worden equity/HWM niet bijgewerkt en alle nieuwe instappen geblokkeerd. Een laatst bekende waarde wordt niet als actueel gepresenteerd.
- Risico per transactie omvat stopafstand, geraamde entry-/exitfees en stopslippage. Gaps kunnen groter zijn. Totale muntblootstelling omvat alle bots, handmatige posities en resterende koopreserves. EUR-only houdt één onderliggende munt in één gecontroleerd paar.
- Een verliesreeks telt afgeronde positielots inclusief entry- en exitfees; gedeeltelijke exits tellen niet voortijdig als een hele transactie.

Verliesstops blijven vergrendeld na herstel van de koers of herstart. Hervatten vereist een expliciete opdracht en nieuwe controles. Een blijvend overschreden cumulatieve limiet of verliesreeks kan in deze versie niet worden gereset. Parameteraanpassing, klonen en nieuwe deployments via de UI zijn fase-3-werk.

## Duurzaamheid, eigenaarschap en herstel

Alle rekening- en botmutaties gebruiken één PostgreSQL-transactie met `SELECT ... FOR UPDATE` op de rekeningrij. Dat serialiseert kapitaal en inventaris over alle bots. De aggregate bevat cash, ledgers, orders, fills, posities, HWM, dagstart, stopstatus, strategiecursor en reeds gebruikte boekliquiditeit. Auditregels worden in dezelfde transactie toegevoegd.

`INTENT` en de reservering worden samen met `PENDING_SUBMIT` gecommit voordat de paperworker de order ziet. Die controleert limieten/data opnieuw. `ACCEPTED` krijgt een aparte commit; pas een volgende cyclus kan vullen. Elke gedeeltelijke fill wijzigt inventaris, cash, fee, resterende reserve en bescherming atomair. Een crash vóór commit heeft geen effect; na commit wordt de opgeslagen state hervat. Een identieke client-id met identieke inhoud geeft hetzelfde order-id; gewijzigde inhoud wordt geweigerd. Een bevestigde preview opnieuw verzenden creëert geen nieuwe order.

State-machine: `INTENT → PENDING_SUBMIT → ACCEPTED → PARTIAL_FILL → FILLED`, daarnaast `PENDING_CANCEL → CANCELED`, `REJECTED` en `UNKNOWN`. Annulering houdt reserves tot de lokale bevestiging. Dubbele fills zijn idempotent; overfills, fills vóór acceptatie en na eindstatus zijn verboden. `UNKNOWN` houdt reserves vast en wordt nooit blind opnieuw verzonden. De lokale adapter heeft geen extern onwaarneembaar order-effect; echte exchange-timeout/reconcile is daarom uitdrukkelijk fase 5.

Een sessiegebonden PostgreSQL-advisorylock laat één paperworker per database toe. Bij verlies van verbinding stopt deze worker; de accounttransactie voorkomt tevens dubbele lokale effecten bij overlap. Dit is geen claim over fencing van een externe livegateway. Connection pool 5, acquisitie 3s, statement timeout 3s, lock timeout 1,5s, idle transaction timeout 5s. Maximum 4 gelijktijdige bedieningsopdrachten, 60 per minuut, body 16 KiB, één replay tegelijk. Geen onbeperkte kritieke orderqueue: opdrachten krijgen direct een fout of een duurzame registratie.

## Bescherming en afbouw

- `PAUSE_ENTRIES`: blokkeert nieuwe blootstelling; bestaande reserves blijven staan. Bestaande kooporders vullen niet verder zolang de pauze actief is. Stops en verkoopintents blijven bewaakt.
- `CANCEL_ENTRY_ORDERS`: pauze plus annulering van resterende kooporders. Geen verwijdering van stops of posities.
- Een verliesstop vraagt automatisch annulering van instaporders aan.
- `EMERGENCY_FLATTEN`: persistente afbouwopdracht, annulering van kooprestanten en begrensde verkopen. De opdracht blijft bestaan als een andere verliesreden zichtbaar wordt. Hervatten kan pas na afbouw.
- Elke entryfill is meteen beschermd voor de werkelijk gevulde hoeveelheid. Bij een stop/target wordt de entryrest geannuleerd voordat afbouw wordt gereserveerd. Een beschermende exit mag niet via een gewone annulering worden verwijderd.
- Verkopen zijn aan een concrete positie en beschikbare, nog niet gereserveerde inventaris gebonden. Verse marktdata en het oorspronkelijke prijsplafond blijven verplicht. Een koersgap kan uitvoering uitstellen. Stofposities onder het orderminimum worden als open en geblokkeerd zichtbaar gehouden.

De bescherming is lokaal gesimuleerd. Bij Mac-slaap of Dockerstop kan de worker niet reageren. Geen exchange-native stop, echte geldgarantie of live-readiness suggereren.

## Paper-fillmodel

Reële OKX `books5`-snapshots leveren bied/laat, hoeveelheden, timestamps en instrumentmetadata. Een fill gebruikt alleen het beste niveau en maximaal 10% van de getoonde hoeveelheid. Verbruikte liquiditeit wordt per instrument/snapshot duurzaam gedeeld over alle orders; een herhaald of ouder snapshot herstelt die capaciteit niet. Orders krijgen deterministisch tijdprioriteit met id als tie-break. Dit is een conservatieve schatting, geen reconstructie van een echte queue.

500 ms gesimuleerde submitvertraging plus de werkelijke worker-/databasevertraging. Een marketorder heeft een vaste prijsgrens. Een limit moet de quote strikt kruisen; aanraking alleen is geen fill. Gedeeltelijke fills, minima, fees en resterende reserves worden verwerkt. Ongevulde instappen vervallen na 24 uur. Nieuwe instappen stoppen bij 5.000 orderrecords of 20.000 fills; afbouw blijft mogelijk. Volledige archivering/retentie volgt later.

**10 bp fee per fill is een expliciete conservatieve aanname**, gelijk voor market en limit. Werkelijke account-/maker-/takertarieven zijn onbekend. Geen accountkoppeling is aangevraagd of verondersteld.

## SMA en replay

`sma-cross-v1`: kies snelle/langzame periode, bedrag, stopafstand en targetafstand. De bot controleert alleen bevestigde, tijdig ontvangen, aaneengesloten candles. Eén positielot per bot; geen martingale of onbeperkt middelen. Een ontbrekende/stale candle wordt als blokkade gelogd. De laatst beoordeelde candle en beslissing worden met de order in één transactie opgeslagen. Na downtime worden geen oude signalen achteraf verhandeld.

Replay gebruikt dezelfde sizing, SMA, limieten, orderstatussen, waardering en bescherming. De adapter leidt geschatte bied/laatprijzen af van echte OHLC, met zichtbare spread/extra-slippage-aannames. Het pad is open → low → high → close. Bij stop én target in één candle krijgt de low/stop voorrang. Een close-signaal kan pas na opening van de volgende candle vullen; de nog open candle wordt uitgesloten. Volume per prijsfase is maximaal 10% van volume/5; dezelfde prijsfase hergebruikt geen verbruikte liquiditeit. Synthetische Heikin-Ashi-chartprijzen komen niet in de executionroute.

Rapport: netto en gerealiseerde P&L, fees, drawdown, turnover, maximale exposure, resterende posities, equitycurve, fills, signalen, parameters en aannames. Resterende posities worden gemarkeerd tegen biedprijs minus exitfees; ze worden niet stilzwijgend tegen een gunstige slotprijs geliquideerd. Alleen recente 300 candles; bulkdatasets, walk-forward, gevoeligheid en Jev-vergelijking volgen in fase 4. Historische simulaties bewijzen geen winst.

## Beveiliging en testgrens

Mutaties vereisen exact dezelfde localhost-Origin als Host en `X-Terminal-Request: paper-v2`; geen CORS. Strict requesttypes weigeren onbekende velden. De preview bevat geen door een client te minten risicotoestemming. Accountrevisie, verloopmoment en prijsgrens worden bij bevestiging opnieuw gecontroleerd. Replay, botbeheer en handmatig handelen kunnen geen liveadapter kiezen.

Nog geen gebruikersauthenticatie, OS-gebruikersisolatie, volledige disk-full/soak/image-securitytest, encrypted backup/restore of exchange-reconcile. Andere lokale processen binnen dezelfde OS-gebruiker zijn een vertrouwde grens. Niet publiek blootstellen.

Implementatie en gebruikte locking/timeoutmechanismen gecontroleerd tegen [PostgreSQL 18 locking](https://www.postgresql.org/docs/18/explicit-locking.html) en [SQLx 0.9 PgConnectOptions](https://docs.rs/sqlx/0.9.0/sqlx/postgres/struct.PgConnectOptions.html#method.options). Uitgevoerde tests en exacte resultaten staan in STATUS.md en evidence/phase2-*.

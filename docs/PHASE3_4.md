# Pine, Bot Studio, alerts en onderzoek · versie 0.4.0

## Pine

Importeer toegestane `.pine`-bron of gebruik het educatieve SMA-voorbeeld. **Preview** voert de echte parser/runtime uit op maximaal 300 recente gesloten, aaneengesloten OKX-candles. AST-inputmetadata levert de parameterformulieren; een onbekende input of waarde buiten min/max wordt geweigerd. Geen handmatige JavaScript-conversie. Bewaar een immutable versie voor gebruik in een bot.

Het profiel gebruikt numerieke `plot(..., "entry")` en `plot(..., "exit")` als **voorstellen**. Positief betekent een voorstel op die gesloten bar; exit gaat voor entry. De bot bepaalt aantallen, stop, target en limieten deterministisch. `strategy.*`, `request.security`, `varip`, imports en onbewezen intrabarsemantiek geven een concrete fout. Zie de volledige compatibiliteitsmatrix.

**Pine-backtest** gebruikt dezelfde geïsoleerde runtime en vaste risk/OMS, maximaal 5.000 geïmporteerde gesloten bars. Een signaal wordt pas vanaf de volgende candle uitvoerbaar. Deze UI-knop gebruikt een zichtbare virtuele startsom €1.000, order €100, stop 1%, target 2%, spread 10 bp, slippage 5 bp en fees 10 bp/fill. Bron, inputs, tijdbereik, aannames en fills staan in het exporteerbare rapport. Geen echte accountfee of tickvolgorde claimen.

## Bot Studio

Een strategie bestaat uit schema 1, nodes en edges. Markt, SMA-cross, RSI-range, Pine, voorwaarden, AND/OR, UTC-tijdvenster, cooldown, nieuws, Jev, sizing, intent en risk-gate zijn beschikbaar. Alleen de topologisch gevalideerde serverconfiguratie wordt uitgevoerd. Maximaal 32 nodes/64 verbindingen; cycli, losse takken, onbekende parameters en routes buiten de verplichte gate worden geweigerd.

Maak eerst een paperbot met expliciete budgetten en stops. Bewaar de grafiek als versie en koppel die aan de **gepauzeerde bot zonder open orders/exposure**. De deployment slaat een exacte versie op. Start/pauze doe je afzonderlijk in Paper. Klonen en een oude versie laden maken nieuwe versies; draaiende strategieën worden niet stil overschreven. Trend/range/Jev-templates zijn educatief, geen rendementsadvies.

Pine/Jev/nieuws leveren alleen signalen. Na elke trage berekening leest de bot opnieuw rekening, versie, status en candlecursor. Alleen `risk::Paper::submit` kan een duurzame paper-intent met reservering maken. Scripts en modellen kunnen limieten niet verhogen.

## Charts en alerts

Chartaantal 1–4, gekoppelde crosshair, tijdzone, indicatorinputs en workspaces staan boven de chart. Sidebarpanelen kunnen links/rechts worden geplaatst; dit is geen vrij dockingsysteem. Tekeningen bewaren prijs-/tijdankers per instrument/timeframe in deze browser. Er zijn horizontale/verticale lijnen, trend, kanaal, Fibonacci, rechthoek, tekst, meten en risk/reward, met magneet, selectie, vergrendeling en undo/redo. Ankerprijzen en kanaalbreedte zijn numeriek bewerkbaar.

Een lijnalert legt een kopie van twee ankers vast en projecteert de lijn server-side op de tijd van de koers/candle-close. Achteraf wijzigen van de tekening verandert een bestaande alert niet. Andere tekenobjecten leveren een expliciet benoemde eerste-ankerprijsalert. Indicatoralerts gebruiken uitsluitend candle-close; prijs/lijn/bot/risk mogen intrabar. Voor botalerts is een bestaande bot-id nodig.

De server controleert ongeveer elke drie seconden. Aflevering is een PostgreSQL-inbox, geen e-mail/push. Bij een overgang onwaar→waar volgt maximaal één melding binnen de cooldown. Maximaal 40 actieve regels. Bij onbetrouwbare/ontbrekende data volgt een foutstatus, geen verzonnen trigger. Meerdere appprocessen dedupliceren via rijlocks. Browser gesloten laat deze loop doorlopen.

## Historie en evaluatie

Importeer 1–10 pagina’s gesloten OKX-candles per opdracht. De import schrijft eerst een immutable Snappy-Parquetbestand en daarna de DB-manifestregel. Hervatten gebruikt de oudere cursor; vernieuwen (`resume:false`) haalt de recente kop op. Download geeft echte PAR1-data. Beschikbaarheid en dekking hangen af van OKX; er is geen onbeperkte tickhistorie.

Walk-forward combineert Parquet en actuele candles (maximaal 10.000), dedupliceert en weigert gaten. Zes SMA-parametercombinaties worden alleen op het trainingsdeel vergeleken. Twee niet-overlappende testvensters volgen chronologisch, met nieuwe virtuele portefeuille per testvenster. Reportages bevatten equity, nettoresultaat, drawdown, fees, turnover, exposure en aannames. Dit blijft OHLC-simulatie met conservatieve stop/target-volgorde.

Vergelijking met/zonder Jev gebruikt werkelijk geregistreerde forward-paperdata. Twee nieuwe bots moeten dezelfde markt, financiële limieten, sizing/stops en grafiek hebben, behalve precies één Jev-filter. Deployments moeten binnen 60 seconden liggen, zonder eerdere orders. AI-kosten blijven in USD naast EUR-handelsresultaten staan; er wordt geen wisselkoers verzonnen. Gedeelde accountlimieten/liquiditeit kunnen beide beïnvloeden: geen causaal of winstbewijs. Echte vergelijking is nog niet uitgevoerd zonder key en expliciete budgetten.

## Jev en nieuws

Model vast `typesafe/jev-1.13`, OpenRouter Decisions API. Geen gewone chatcompletion of automatische fallback. Drie modelkeuzes worden genormaliseerd in een intern envelope; werkelijk ontvangen confidence/probabilities/kosten worden behouden. Confidence is geen winstkans.

Account-/botkostenlimieten, één gelijktijdige aanvraag, vooraf $0,005 reserve, 2,5 s deadline en een breaker na drie mislukkingen beschermen de route. Onbekende kosten houden hun reserve. Te laat, lage zekerheid (<0,8), gewijzigde rekening/candle/strategie of ongeldige output betekent geen AI-afhankelijke instap. Bescherming blijft zonder AI werken. Kostentelling per bot-id overleeft strategiewijzigingen.

OKX-announcements/status en Alternative.me draaien buiten de orderroute. Bron-URL, publicatie/ontvangst en TTL blijven zichtbaar. De dagelijkse index gaat over Bitcoin. De regionale announcementsroute gaf bij acceptatie geen berichten; de UI verzint geen vervanging. CryptoPanic/CoinGecko en DeMARK blijven beperkt door nog niet vastgestelde providerrechten/licenties.

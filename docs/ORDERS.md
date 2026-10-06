# Native OKX-orders · 6 oktober 2026

De Terminal heeft een handmatig orderticket met gescheiden OKX Demo- en Live-administratie. Op uitdrukkelijk verzoek van Willem is op 5 oktober 2026 ook de handmatige Live-route gebouwd, met een persoonlijke vrijgave per account en limietversie. Een vrijgave is geen bewijs dat alle productacceptatie is afgerond. De netwerkloze Pine-worker en bestaande paperbots plaatsen geen native orders. Het ticket maakt eerst een immutable preview van 15 seconden. Alleen de vaste orderrisicokern kan een uitvoeringsmachtiging uitgeven. Een afzonderlijke `okx-orders`-gateway heeft exclusief eigenaarschap en is de enige nieuwe private uitvoeringsroute.

## Gebruik

1. Start de terminal zoals gebruikelijk. De afzonderlijke Demo- en Live-gateways verbinden met hun eigen key. Saldi worden opgehaald zonder handelsvrijgave; een aansluiting plaatst geen order.
2. Vul zelf de API-key, secret en passphrase in bij **Systeem → Aansluitingen & AI → OKX Demo** of **OKX Live**. Gebruik een EEA-account met read/trade-rechten. De gateways gebruiken uitsluitend de vaste EEA-host. Alleen Demo krijgt de OKX-simulatieheader. Nooit keys in chat zetten.
3. Klik bij **Nog nodig voor Live-handelen** op **Account en handelslimieten instellen**, of open **Terminal → Order plaatsen → Account en handelslimieten**. Controleer het gedetecteerde account. Kies zelf toegestane EUR- en USDC-spotmarkten, budget, order-/munt-/verlieslimieten en frequentie. Na invoer van je handelsbudget verschijnen aanpasbare voorbeeldlimieten. Controleer ze en bevestig zelf; het formulierconcept slaat niets automatisch op.
4. Bij Live bevestig je daarna **Live vrijgeven** voor het gedetecteerde account en precies deze limietversie. De aansluiting moet actuele handelsrechten, twee private kanalen en een gereconcilieerde orderadministratie hebben. Gewijzigde limieten trekken de vrijgave in. Een tijdens de actieve gateway vervangen key trekt haar eveneens in; ieder account wordt opnieuw aan de bevestigde identiteit getoetst.
5. Kies kopen/verkopen en het ordertype. Hoeveelheden zijn altijd in de basismunt. Nieuwe aankopen vereisen een beschermende stopprijs. Klik **Order controleren** en lees de exacte preview.
6. Bevestig zelf de order. De wachtrij, exchange-ACK en bevestigde fills zijn verschillende statussen. **Mijn OKX-orders**, **Uitvoeringen en fees** en **Bescherming** tonen de administratie voor de gekozen omgeving.

## Ordertypen

| Ticket | Native OKX-type | Instellingen |
| --- | --- | --- |
| Markt | market | Basishoeveelheid; buy-slippage 0–5%; banAmend |
| Limiet | limit | Prijs en totaalhoeveelheid |
| Post-only | post_only | Alleen maker |
| IOC | ioc | Direct uitvoerbaar deel, restant annuleren |
| FOK | fok | Volledige hoeveelheid of annuleren |
| Trigger | trigger | Laatste handelsprijs; markt, limiet, IOC of FOK na trigger |
| TP/SL | conditional of oco | Losse TP of SL; beide samen altijd OCO |
| Trailing | move_order_stop | Percentage óf prijsafstand; optionele activatieprijs |
| Iceberg | smart_iceberg | Totaal, deelgrootte, zichtbare delen, agressiviteit, prijsgrens, direct/prijs/RSI-start |
| TWAP | twap | Totaal, deelgrootte, prijsgrens, secondeninterval, percentage óf prijsafstand |

De implementatie volgt de actuele [OKX-algo-API](https://www.okx.com/docs-v5/en/#order-book-trading-algo-trading-post-place-algo-order). Beschikbaarheid en strengere minima hangen van account, regio en Demo af. Er is geen fallback naar een ander account of instrument.

## Uitvoering en bescherming

- Alle bedragen gebruiken Decimal. De risicokern reserveert de volledige Iceberg-/TWAP-hoeveelheid. Voor verkopen telt ook inventaris mee die voor native bescherming is gereserveerd.
- Voor verzending zijn verse rekeningdata, actuele fees, een gesynchroniseerd account en bevestigde private/business-kanalen nodig. De adapter onderhoudt orders/account en orders-algo/algo-advance; events wekken de REST-reconciler. Native children worden afzonderlijk gevolgd.
- Intents worden vóór verzending opgeslagen. Een timeout of crash tijdens submit wordt UNKNOWN. De gateway vraagt eerst de native status op via de oorspronkelijke client-ID en verzendt nooit blind opnieuw.
- Fills worden uniek op OKX bill-ID opgeslagen, met prijs en feevaluta. Basefees verlagen de werkelijk verkregen hoeveelheid. Een triggerstatus `effective` is geen fill.
- OKX aangehechte stops worden pas actief bij volledige parentfill. Deze gateway maakt daarom afzonderlijke native SL/OCO-orders voor bevestigde, fee-adjusted filldelen. Hij toont bescherming pas actief na een detailcontrole. UNKNOWN/reject, een verdwenen stop of een deel onder de minimumgrootte blokkeert nieuwe aankopen. Parentannulering wordt aangevraagd bij een mislukte beschermingsaanvraag; resterende blootstelling vraagt beoordeling bij OKX. Er is geen gegarandeerde verliesgrens.
- Native stops blijven bij OKX staan wanneer de lokale app stopt. Nog niet beschermde fills blijven een acceptatierisico. Herstart reconcilieert de bestaande beschermingsidentiteiten en houdt onzekere claims vast.
- Boekingen, fees, EUR-stortingen/-onttrekkingen en risicogeheugen blijven lokaal bewaard. Onverklaarde cashflows/externe transacties blokkeren nieuwe aankopen. Alleen actuele EUR-waardering is ondersteund; volledige ledger-acceptatie blijft vereist.
- Annuleren vóór verzending gebeurt atomair lokaal. Na verzending blijft een reserve staan tot de definitieve native status.

## Huidige grenzen en vrijgave

Dit is geen volledige pariteit met alle OKX-producten. Het profiel is **EUR- en USDC-spot zonder hefboom, hoeveelheid in basismunt**. Alle door het eigen OKX-account teruggegeven markten met deze betaalvaluta’s kunnen persoonlijk worden bevestigd (maximaal 2000). Margin, futures/perpetuals, quote-hoeveelheden, amend/replacement, chase, VWAP en gesplitste TP-ladders zijn niet geïmplementeerd. Een bestaande order kan worden geannuleerd; een andere order krijgt een nieuwe preview. De paper-/Pine-bots blijven paper.

Een triggerkoop met marktuitvoering, TP/SL-koop en trailing-koop hebben geen vaste maximale toekomstige aankoopprijs. De risicokern blokkeert deze nieuwe blootstelling; ze kunnen wel voor verkoop van bestaande inventaris worden voorbereid. Trigger-limit en prijsbegrensde Iceberg/TWAP zijn de ondersteunde gesplitste/uitgestelde kooproutes.

**Handmatige Live-vrijgave is beschikbaar na persoonlijke account-, instrument-, budget- en limietbevestiging en een actuele verbindingscontrole.** De control plane en de afzonderlijke Live-gateway controleren diezelfde vrijgave opnieuw. Pauzeren weigert QUEUED-opdrachten en houdt bestaande native orders en stops intact. Onbekende uitkomsten houden reserveringen vast. Pine en bots blijven PAPER_LOCAL.

De eerdere algemene Live-blokkade is voor deze handmatige route vervangen op Willems expliciete verzoek. Volledige fase-5-acceptatie en echte Demo/Live-orderacceptatie zijn niet ineens afgerond: er is geen echte order geplaatst om de vrijgave te testen. Met name partial-fillbescherming, geavanceerde childhistorie, cashflows, uitval/herstel en alle gewenste native ordertypen vereisen nog echte acceptatie. `live_acceptance_complete` en `live_technical_accepted` blijven daarom false. De persoonlijke vrijgave heeft een afzonderlijke betekenis en administratie.

Nog onafhankelijke implementatie voor volledige fase 5: één gezamenlijke paper/native/bot-ledger; volledig cashflow- en positieherstel; amend/replacement; automatische gecontroleerde afbouw en beschermingsvervanging; historische reconciliatie van grote orderlijsten en definitief afhandelen van onbekende uitkomsten. Zie ook docs/PHASE5_EN_AFRONDING.md. Deze beperkingen zijn niet alleen een ontbrekende key.

## Herhaalbare controles

- Rust-suite: `rtk docker build --target verification -t okx-orders-verification:0.4.0 .`
- Productiebuild: `rtk docker build -t okx-tradingterminal:0.4.0 .`
- Geïsoleerde HTTP/DB-test: `rtk proxy python3 scripts/order_acceptance.py provision`, daarna `check` en `cleanup`. De test gebruikt een nieuw opgenomen DB-/containerpaar op poort 4781, synthetische accountgegevens en echte publieke prijzen. Nul private OKX-calls. Hij weigert een bestaand manifest.


## Saldi en percentages

- **Mijn OKX-saldi** toont alle teruggegeven munten, waaronder BTC, USDT en EUR, met cashsaldo, vrij en vastgezet saldo. Handelsrekening en Funding staan apart. Funding wordt nooit stilzwijgend overgezet of gebruikt voor een order.
- De saldoroute gebruikt uitsluitend GET: accountconfiguratie, handelssaldo en Funding-saldo. Leesrechten volstaan. Een ontbrekend veld is onbekend, geen nul. Een mislukte Funding-call verbergt de actuele handelsrekening niet.
- De slider loopt van 0 tot 100%, met 25/50/75/100-knoppen. Verkoop gebruikt de vrije basismunt, koop de vrije betaalvaluta van de markt (EUR of USDC). Lokale order- en beschermingsclaims worden conservatief afgetrokken; een native blokkering kan daardoor dubbel tellen en een te kleine hoeveelheid geven.
- Koophoeveelheden gebruiken de opgegeven maximumprijs of markt-slippagegrens en de actuele private accountfee. Er zijn geen geschatte standaardfees. De server rekent met Decimal en rondt naar beneden op de lotgrootte.
- Onder de minimumhoeveelheid, bij onbekend saldo of na 15 seconden zonder saldovernieuwing ontstaat geen uitvoerbare percentageorder. Bij controleren wordt de percentagehoeveelheid opnieuw berekend en in de exacte preview getoond. Een bevestigde preview wordt nooit stilzwijgend opgeschaald.
- Omgevings-, account-, richting- en marktwijzigingen wissen de gekozen percentagehoeveelheid. Handmatige invoer schakelt de percentageselectie uit.
- Het profiel ondersteunt EUR- en USDC-spot. USDT-saldo is zichtbaar; USDT-markten en automatische valutaomwisseling vallen buiten dit profiel.

API: [OKX handelssaldo](https://www.okx.com/docs-v5/en/#trading-account-rest-api-get-balance) en [OKX Funding-saldo](https://www.okx.com/docs-v5/en/#funding-account-rest-api-get-balance).


## Voorbeeldlimieten na invoer van handelsbudget · 5 oktober 2026

Het budgetveld vult de overige limieten in met één aanpasbaar startprofiel. Dit is een productvoorbeeld, geen vastgesteld gemiddelde van traders. Bij EUR 500: order EUR 50 (10%), blootstelling per munt EUR 125 (25%), risico tot de stop inclusief fees EUR 5 (1%), dagverlies EUR 15 (3%), totaalverlies en drawdown ieder EUR 50 (10%), drie open orders, twee orders per minuut en 20 basispunten spread (0,2%). Deze voorbeeldpercentages en aantallen zijn expliciete ontwerpkeuzes. [CME beschrijft percentagegrenzen als een keuze](https://www.cmegroup.com/education/courses/trade-and-risk-management/the-2-percent-rule), niet als universeel optimum.

Automatische geldvelden rekenen exact met het nieuwe budget mee. Zelf aangepaste of bewust leeggemaakte velden blijven behouden. **Startprofiel opnieuw invullen** vervangt eigen waarden uitsluitend in het formulierconcept. Opgeslagen limieten worden bij openen geladen en als eigen keuzes behandeld. Een budget-/veld-/marktwijziging wist de persoonlijke bevestiging. Een account- of omgevingswissel laadt het eigen beleid van die rekening en neemt het vorige concept niet over.

De berekening gebruikt gehele getallen en decimale strings. Zij rondt geen limiet omhoog en verandert geen order-/risicomachtigingen. Opslaan blijft de bestaande gevalideerde route; marktkeuze, eigen bevestiging en afzonderlijke Live-vrijgave blijven vereist. Een nieuw budget maakt geen geld beschikbaar. Er is geen automatisch opslaan of native order door het invullen van een voorbeeld.


## Limietprijs uit een grafiekklik · 5 oktober 2026

Kies **Kopen** of **Verkopen** en een **Limietorder**. Klik in het koerspaneel, met de tool **Selecteren** actief. De prijs op die verticale hoogte verschijnt bij **Limietprijs**, afgerond op de dichtstbijzijnde OKX-prijsstap. Dit werkt in alle gekoppelde grafieken voor de geselecteerde markt, ook op verschillende tijdframes. Een melding onder het veld bevestigt de overgenomen prijs.

Post-only, IOC en FOK gebruiken hetzelfde veld. Bij een trigger met limiet-/IOC-/FOK-uitvoering verandert alleen **Orderprijs na trigger**; de triggerprijs blijft behouden. Marktorders, TP/SL, trailing, Iceberg en TWAP worden door de klik niet aangepast. Klikken in volume, indicatoren, assen of tekenbediening en het slepen van de grafiek wijzigen de limietprijs niet. Een tekening plaatsen of verplaatsen blijft een aparte handeling.

De hoeveelheid en richting blijven behouden. Een eerder met de slider berekende hoeveelheid wordt bij een grafiekklik een vaste handmatige hoeveelheid; kies opnieuw een percentage als je haar met de nieuwe prijs wilt herberekenen. Bij wisselen van markt worden hoeveelheid en limietprijs gewist. Klikken maken geen orderpreview, uitvoeringsmachtiging of native order. De bestaande controle en persoonlijke bevestiging blijven gelden.

Verificatie: vier gerichte tests voor exacte tickafronding, decimalen/exponenten, ongeldige prijzen en juiste ordertypen, plus zeven limieten- en elf tekenmodeltests. TypeScript/productiebuild en browserproef slagen. De browserproef controleert koop/verkoop, twee prijshoogten, volume/RSI/slepen, tekenen met undo, percentagebehoud, geen latere toepassing van een marktklik, triggerbehoud, marktreset en tweede grafiek. Bewijs: `evidence/chart-price-*`. Backend en gateways zijn ongewijzigd; echte native orderacceptatie blijft een afzonderlijke open controle.


## EUR/USDC-uitbreiding · 6 oktober 2026

Willem heeft de uitbreiding naar alle EUR- en USDC-spotmarkten van zijn eigen OKX-account expliciet toegestaan. De actuele bron is gericht samengevoegd en de terminal plus beide afzonderlijke ordergateways zijn bijgewerkt. De eerdere grens van twaalf persoonlijk gekozen markten is verwijderd. Budget, limieten, acht gekozen EUR-markten en de bestaande persoonlijke Live-vrijgave zijn behouden; geen nieuwe markt of order is namens Willem bevestigd.

Het account-endpoint blijft de bron voor beschikbare markten. **Alle beschikbare spotmarkten kiezen** selecteert de volledige EUR/USDC-accountlijst in het formulierconcept. De gebruiker bevestigt vervolgens zelf de limieten en geeft Live opnieuw vrij voor die nieuwe versie. Prijsvelden, previews en kooppercentages gebruiken de werkelijke betaalvaluta. EUR-budget en risico gebruiken actuele marktconversies. Een ontbrekende/verouderde conversie of exacte marktfee blokkeert de aankoop. Kopen met USDC en verkopen van USDC delen dezelfde saldoreservering; crypto-inventaris en blootstelling worden over EUR/USDC-paren van dezelfde munt samengenomen.

**Actuele accountcontrole is nog geblokkeerd:** OKX antwoordt HTTP 401 / code 50110 op `/api/v5/account/bills`, en de private login wordt eveneens geweigerd. De terminal toont dat het huidige IP-adres niet door de API-key wordt toegelaten. De IP-lijst moet door Willem bij OKX worden gecontroleerd. Er is geen verse EUR/USDC-accountlijst geclaimd: de oude lijst van 270 EUR-markten is verouderd. De gateway blijft gepauzeerd en de UI toont geen onjuiste marktweigering of actieve Live-verbinding op basis van dat oude snapshot.

Bewijs: productiebuild en TypeScript; 211 Rust-tests (32 onboarding, 44 gateway, 135 terminal); 32 bestaande native HTTP-controles en 25 aanvullende EUR/USDC-controles in een wegwerpdatabase. De tweede productiebrowserproef bevestigt USDC-prijsunits, grafiekklik met hoeveelheidbehoud, de IP-melding, en geen consolefouten. De fictieve browserrekening bevestigt wissen/alle kiezen, USDC-zoeken en uitgeschakelde opslag zonder eigen bevestiging. De sliders blijven in de browser uitgeschakeld zonder geconfigureerde keys; hun 50%-USDC-berekening is via de geïsoleerde HTTP-route getest. Geen echte testorder, transfer of gebruikersinstelling gewijzigd. Dit is geen volledige native exchange- of fase-5-acceptatie.

Bewijsbestanden: `evidence/eur-usdc-*`. Herhaalbare controles: `scripts/order_acceptance.py provision`, `scripts/eur_usdc_checks.py`, daarna `scripts/order_acceptance.py cleanup`. De opname verwijst uitsluitend naar de wegwerpomgeving op poort 4781.

## Markttoegang en diagnose · 5 oktober 2026

Deze sectie beschrijft de historische stand vóór de expliciet toegestane uitbreiding van 6 oktober.

De publieke catalogus is nu volledig doorzoekbaar: bij controle 1143 live spotparen, waaronder 271 EUR- en 298 USDC-paren. Publieke zichtbaarheid is geen accounttoestemming. Grafieken buiten de oude twaalf paren worden op verzoek gekoppeld aan de bestaande publieke boek-/ticker-/tradevalidatie; actieve feeds blijven begrensd. Browserproef bevestigt NEAR-EUR met actuele candles en boek.

Het orderticket onderscheidt een beschikbare EUR-markt die niet persoonlijk is bevestigd van de terminalbeperking op een andere betaalvaluta. Het limietenformulier toont de EUR-markten uit de lopende accountgateway met een zoeker. De huidige grens van twaalf tegelijk blijft staan. Bij controle waren acht EUR-markten persoonlijk bevestigd en gaf de lopende, nog EUR-filterende gateway 270 EUR-paren terug. De publiek zichtbare andere valuta's worden niet ten onrechte als ontbrekende OKX-rechten voorgesteld.

De brede Live-/risico-uitbreiding werd door automatische goedkeuringscontrole geweigerd wegens ontbrekende specifieke markt-/valutatoestemming. Een diagnostische gatewayherstart werd afzonderlijk geweigerd wegens mogelijke verstoring van de Live-service. Daarom is uitsluitend de terminal vernieuwd; beide private gateways bleven draaien. De wijziging om de volledige accountlijst te lezen staat in de gatewaybron maar is daar nog niet actief. De ordercompiler, wallet- en risicokern blijven EUR-only. De gerichte EUR/USDC-uitbreiding is apart voorbereid en geïsoleerd typechecked/getest; zie [het concrete voorstel](MARKET_ACCESS_PROPOSAL.md).

De productiebuild, TypeScript, 191 bestaande Rust-tests en vier chartprijstests slagen. Browsercontrole bevestigt publieke zoekbaarheid, live feed voor NEAR-EUR, de melding over ontbrekende persoonlijke marktbevestiging, directe formulierroute, NEAR ongekozen, acht opgeslagen markten en uitgeschakelde bevestiging. Het USDC-ticket meldt de terminalbeperking. Geen consolefouten of -waarschuwingen. Beleid en persoonlijke Live-vrijgave zijn behouden. Bij controle stond één volledig uitgevoerde native BTC-EUR-verkoop in de administratie; de agent heeft geen order/preview aangemaakt of bevestigd. Dit ene gebruikersresultaat is geen volledige native acceptatie. Bewijs: `evidence/market-access-*`.

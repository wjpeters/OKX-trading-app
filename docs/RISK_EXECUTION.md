# Risicokern en uitvoeringscontract

De fase-2-kern werkt voor PAPER_LOCAL. De exacte geïmplementeerde grondslagen, statusovergangen, beperkingen en herstelregels staan in [PHASE2.md](PHASE2.md). Onderstaand contract blijft het ontwerp voor toekomstige exchange-uitvoering; claims over private reconcile/live-fencing zijn nog niet geïmplementeerd.

## Autoriteit

1. Marktstate, posities en fees deterministisch berekenen.
2. Eerste gate voorkomt verboden/overbodige AI-aanroepen.
3. Jev kiest alleen uit vooraf toegestane proposities. Geen credentials, tools, sizing, nieuwe limieten of eigen orderpayload.
4. Een DecisionEnvelope koppelt providerantwoord aan decision-id, snapshot-id, strategieversie, gepind model, werkelijke probabilities/confidence, tijden, TTL en kosten. Geen verzonnen redenering of confidence.
5. Alleen de risicokern maakt een begrensde order-intent. Account- én botlimieten verplicht en expliciet bevestigd.
6. Eén serializable PostgreSQL-transactie vergrendelt account/bot/valuta, reserveert geld/inventaris, schrijft intent plus unieke client-order-id en audit, en commit vóór verzending.
7. Direct vóór verzending valideert de gateway dezelfde limieten, lease/fence, dataversheid, snapshot- en accountversie. Een conflict betekent opnieuw beoordelen, geen blind retry.

Geen directe route browser→OKX, Pine→OKX of Jev→OKX. Handmatig, bot en script delen deze gate. De interne AuthorizedIntent heeft een niet-publiek construeerbare capability en een korte TTL; geen algemene HTTP-header waarmee de UI zichzelf kan autoriseren.

## Rekengrondslagen (ontwerp, bevestiging bij onboarding)

Start spot zonder leverage. Accountvaluta expliciet kiezen; EUR is een voorstel. Prijs, sizing, tick/lot, fees, reserves: Decimal/fixed-point. Waardeer bezittingen tegen uitvoerbare conservatieve biedprijs minus geraamde exitfees; tegenpartij-/conversion-data moet actueel zijn. Open kooporders reserveren hun maximale kosten plus fee/slippagebuffer. Verkopen reserveren inventaris. Reserves mogen vrije ruimte verlagen maar niet opnieuw van equity worden afgetrokken als ze al onderdeel van de balans zijn.

Bewaar netto equity E(t) inclusief gerealiseerde/on-gerealiseerde P&L en alle fees/funding. Externe netto cashflows C(t) worden apart gejournaliseerd. Cashflowgecorrigeerde equity A(t)=E(t)-C(t), met dezelfde baseline voor de meetperiode. Dagverlies=max(0,A(dagstart)-A(t)); dagverliespercentage deelt door positieve A(dagstart), anders entries blokkeren. Voor toekomstige live-onboarding was Europe/Amsterdam voorgesteld. PAPER_LOCAL gebruikt in fase 2 expliciet bevestigde 00:00 UTC, zie PHASE2.md.

Cumulatief verlies gebruikt een vastgelegde startbaseline; reset vereist expliciete menselijke controle. High-water-mark H(t)=max(H(vorig),A(t)); drawdown=max(0,H(t)-A(t)), percentage /H(t) voor positieve H. Bewaar dagstart, baseline, cashflowledger, HWM en stopstatus in dezelfde duurzame namespace. Herstart reset nooit verlieslimieten. Accountbrede totale equitydaling begrenst alle bots plus externe/handmatige orders.

Alle limieten uit §10 blijven verplicht: toewijzing, risico/transactie, dag/cumulatief verlies, drawdown, positie- en onderliggende exposure, aantallen posities/orders, leverage, spread/slippage, ouderdom, frequentie, verliesreeks en AI-kosten. Geen defaults die ongemerkt live toestemming geven. Virtuele botbudgetten isoleren niet zoals echte subaccounts.

## Storingsbeleid

| Storing | Actie |
|---|---|
| Verse marktstate ontbreekt, Jev timeout/ongeldig/te laat, AI-budget bereikt | PAUSE_ENTRIES voor betreffende strategie/instrument; deterministische bescherming blijft |
| Verliesdrempel, onbekende accountstate | PAUSE_ENTRIES + CANCEL_ENTRY_ORDERS volgens bevestigde policy; beschermingsorders behouden |
| Unknown-order, DB/diskfalen, leaseverlies | Geen nieuwe verzending; reservering vasthouden; status reconciliëren |
| EMERGENCY_FLATTEN bewust gevraagd / vooraf bevestigd beleid | Beperkte afbouw; behoud bescherming tot sluiting of vervanging bevestigd |

Slippageplafond en gegarandeerd onmiddellijk sluiten kunnen niet tegelijk gegarandeerd worden. Loss limits zijn actiedrempels. Voor hervatten zijn gebruikercontrole en nieuwe verse state nodig; bot/AI kan geen limiet verhogen.

## OMS en isolatie

INTENT → PENDING_SUBMIT → ACCEPTED → PARTIAL_FILL → FILLED. PENDING_CANCEL → CANCELED; REJECTED alleen na bevestigde afwijzing. Een verzendtimeout wordt UNKNOWN, niet automatisch retry. Dedup fill-ID, events mogen out-of-order zijn, acknowledgement is geen fill. Partial fill geeft alleen werkelijk onbenut reservebudget vrij. Voor startup: snapshot + private updatebuffer, balances, pending orders en recente fills reconciliëren; externe orders meenemen.

PAPER_LOCAL, OKX_DEMO en LIVE krijgen afzonderlijke binaries/adapters, schema/namespace, credentials en deployment-id. Paper bevat geen live-adapter. Actieve exposure kan niet met toggle van omgeving veranderen. Alleen de actieve livegateway bezit key/egress plus accountfence. Cancel All After annuleert binnen scope en sluit geen posities; beschermingsgevolgen vooraf testen.

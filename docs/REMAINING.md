# Resterend werk tegen de volledige bouwprompt

Zie het [uitvoeringsplan van 4 oktober 2026](PHASE5_EN_AFRONDING.md) voor de volgorde, mijlpalen en acceptatie per werkpakket. De onderstaande hiaten blijven open totdat het bijbehorende bewijs is geleverd.

De opdracht is de app af te bouwen. Versie 0.4.0 is een werkende lokale paper-/onderzoeksrelease, **geen afgeronde volledige fase 5**. Alle hoofdstukken van BUILD_SPEC.md blijven gelden. De onderstaande implementatie ontbreekt nog; dit is niet uitsluitend een kwestie van keys invullen.

## Exchange-uitvoering

1. Verifieer met een nieuwe EEA Demo-key de accountregio, toegestane instrumenten, eigen fees, private streams en native stopmogelijkheden. Gebruik de gebouwde read-only onboardingtool. Geen key in chat of oude projectkeys gebruiken.
2. Bouw continue snapshot+private-update-reconciliatie voor balances, posities, orders en fills, inclusief externe/manual orders en cashflows. De huidige bounded kanaalprobe is geen synchronisatie-engine.
3. Verbind dezelfde vaste riskkern aan een afzonderlijke duurzame Demo-orderadapter/outbox. Verwerk werkelijke fees (ook basecurrency), partial inventory, ACK versus fill, pending cancel/amend, unknown-submit en reconcile vóór retry. De bestaande private transportsigner is geen vrijgegeven ordergateway.
4. Implementeer/accounttest native stops na partial fills/amends, veilige replacement, inventarisbegrensde spotafbouw en de gevolgen van Cancel All After. Reserveer requestcapaciteit voor bescherming.
5. Dwing één eigenaar af met gatewaylease én effectieve credential/egress-fencing. Een PostgreSQL-advisorylock alleen beschermt geen twee onafhankelijke live-deployments.
6. Doorloop de volledige exchange-foutmatrix: acceptatie gevolgd door timeout/crash, dubbele/out-of-order events, lost cancel/amend, private disconnect, rate-limit, storage failure, leaseverlies en bescherming na herstel. Een fixture is geen echte Demo-acceptatie.
7. Pas daarna een afzonderlijk LIVE-profiel, opslagnamespace en releasebewijs. Account, instrumenten, budget en alle limieten expliciet door Willem bevestigen. In deze versie ontbreekt die uitvoering en blijft LIVE technisch geblokkeerd.

## Verdere productscope

- Volledig docking, interactieve orderlijnwijziging en beveiligde snelhandelsmodus. Nu wel multi-chart, werkspaces, lijnweergave en preview/bevestiging.
- Verdere aantoonbare Pine-profielen voor strategy.*, intrabar/rollback/varip, eigen tekenobjecten en request.security met gaps/lookahead. Geen stilzwijgende polyfills.
- Business-WS-candles, history-trades en bulk tick/L2-import met werkelijke coverage; candle-replay vervangt geen tick/orderbook-simulatie.
- Voor volume profile/orderflow/heatmaps eerst beschikbare brondata vaststellen. DeMARK alleen met toegestane regels/licentie en referentiegevallen.
- CryptoPanic/CoinGecko pas na plan/rechten/schema/quota. Echte Jev-respons- en kostentests plus forwardvergelijking na key/budgetbevestiging.
- Langdurige soak/burst en volledige event→intent/submit→ack/fill-percentielen. amd64 werkelijk draaien; nu Linux arm64 uitgevoerd. Onafhankelijke securityreview, MFA/private VPN/TLS pas voor een afzonderlijk geautoriseerde remote inrichting.
- Automatische backupretentie/streaming voor >256 MiB, browsertekeningen in serverbackups, en herstel naar een vervangende productiehost. Huidige backup/restore test is lokaal/disposable.

Geen van deze open punten wordt gepresenteerd als mockknop of afgeronde functie. Gebouwde en werkelijk uitgevoerde onderdelen staan in STATUS.md en evidence/.

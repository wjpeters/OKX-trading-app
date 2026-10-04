# Pine-compatibiliteit — profiel closed-bar-1

Runtime: PineTS 0.11.0 (AGPL-3.0-only, gepind in pine-worker/package-lock.json). Dit project is alleen lokaal en privé. Distributie of beschikbaarstelling aan anderen vraagt een afzonderlijke licentiebeoordeling. De upstream-runtime is vervangbaar achter studio::run_pine; nooit een gateway-afhankelijkheid.

| Functie | Terminalprofiel |
| --- | --- |
| Echte Pine v5/v6-bron, parser en AST | PineTS-parser; geen eigen regex-transpiler |
| Series, history, na, inputs, var | Closed-bar preview; fixtures in pine-worker/tests.mjs |
| SMA/EMA/RSI en overige TA | Via gepinde runtime, geen claim dat alle upstream-functies zijn gevalideerd |
| Control flow en collecties | Runtime, met maximum 10.000 loopiteraties en procesdeadline |
| Plots | Maximaal 20 numerieke reeksen; tekenobjecten van Pine worden niet weergegeven |
| entry/exit-plots | Onvertrouwde voorstellen; alleen vaste risicokern kan paperorders toestaan |
| Strategy.* | Concrete fout; geen runtimefills als echte terminalfills interpreteren |
| request.security, import | Concrete fout; er is nog geen gevalideerde multi-timeframe-data-adapter |
| Open bars, varip, intrabar/rollback | Geblokkeerd in dit profiel, nog niet voor handel vrijgegeven |
| Volledige TradingView-compatibiliteit | Niet geclaimd |

Elke run krijgt alleen aaneengesloten gesloten OKX-bars. Geen toekomstige bars, externe providers of synthetische chartprijzen. De bron wordt eerst op verboden capabilities gecontroleerd en daarna door de echte PineTS-parser verwerkt. Bij iedere fout worden alle plots/intents verworpen. Een gepinde scriptversie kan via een Pine-node aan een paperbot worden gekoppeld; de server blijft zonder browser draaien.

Beveiliging: afzonderlijke non-root container, network_mode none, geen database-/credentialmounts/hostbestanden/Docker-socket, read-only root, 256 MB containergeheugen, 0,5 CPU, 32 processen. Iedere run is een nieuw Node-proces met permission model, alleen leesrecht op runtimecode, 128 MB heap, 3 s deadline, 1 MB maximale uitvoer. Broker verwerkt maximaal vier gevonden jobs per cyclus; backend laat één script tegelijk toe. Jobbestanden verlopen en worden verwijderd.

De editor bouwt inputformulieren uit AST-metadata, inclusief defaults/min/max. Overrides worden gevalideerd en samen met de immutable scriptversie opgeslagen. Pine-candle-backtest en paper gebruiken dezelfde worker; replay geeft voorstellen alleen vanaf de volgende bar aan dezelfde vaste risk/OMS. Historische datasets komen uit echte Parquet/OKX-bars, maximaal5000 voor Pine. Elf concrete fixtures staan in evidence/pine-tests.json. Dat corpus bewijst het geteste profiel, niet alle upstream-Pinefuncties.

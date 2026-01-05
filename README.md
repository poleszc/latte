# Latte - Piotr Oleszczuk, nr 448390
## Co jest zrobione
- Pełna podstawowa wersja zadania z backend'em LLVM
- Dodatkowo z rozszerzeń (zarówno we frontendzie i backendzie):
  - Użycie rejestrów i phi zamiast alloca (aktualnie w generowanym LLVM IR pojawiają się jeszcze instrukcje typu `%r0 = bitcast i32 10 to i32`, czyli odpowiednik `%r0 = 10` działający nie tylko dla int'ów. To oczywiscie zniknie po pierwszym passie constant propagation)
  - Tablice (bez tablic tablic, nie do końca jest dla mnie jasne, czy powinny one być)
  - Struktury
  - Obiekty
  - Metody wirtualne
  - Zagnieżdżone struktury, obiekty, tablice struktur, obiekty z tablicami itp.
  - Jawna obsługa błędów wykonania takich jak dzielenie przez 0, null ptr access (poprzez wywołanie `error()` z `runtime.ll`)
- Optymalizacje:
  - Constant Propagation i Constant Folding
  - Dead Code Elimination
  - Branch Folding - zamienianie instrukcji typu `br [stała_logiczna_znana_w_czasie_kompilacji], blok1, blok2` na odpowiedni `jump`
  - Dead block elimination - usuwanie nieosiagalnych bloków powstałych w wyniku Branch Folding

## Budowanie projektu
1) Instalacja Rust:
   - Aby zainstalować należy uruchomić skrypt `build.sh` znajdujący się w katalogu głównym projektu.
    Skrypt instaluje Rust i Cargo (package manager) z oficjalnej strony Rusta i ładuje zmiany
    żeby były dostępne z aktualnej sesji terminalowej, a następnie wykonuje polecenie `make`, które
    pobierze wszystkie potrzebne dependencje, zbuduje projekt i przekopiuje pliki wykonywalne
    do katalogu głównego.
2) Budowanie projektu:
   - Projekt sam skompiluje się po wykonaniu skryptu `build`. Jeżeli jednak zajdzie potrzeba ponownej
    kompilacji (pomijajac już samą instalacje Rust) w katalogu głównym projektu trzeba uruchomić `make`,
    który skompiluje projekt i przekopiuje pliki wykonywalne `latc_llvm` oraz `insc_llvm` do katalogu głównego.
    - **Dodatkowe opcje:**
      - `make build-llvm` - przebuduje jedynie binarkę z backendem LLVM
      - `make clean` - usuwa zbudowane binarki oraz pliki `*.{mir, lir, tast.json, json, ll, bc}` z katalogu `tests/in`
      - `make clean-all` - usuwa to co `make clean` oraz wszystkie pliki dependencji


## Użycie kompilatora
### Podstawowe użycie
```bash
# Uruchomienie `latc_llvm` na pliku z rozszerzeniem `.lat` wykona analize semantyczną i kompilacje do LLVM. W lokalizacji pliku źródłowego pojawi się plik `*.ll` z LLVM IR kompilowanego kodu, plik `*_raw.bc` z bitkodem kompilowanego pliku oraz `*.bc` z bitkodem skompilowanego pliku zlinkowanym z biblioteką `runtime.ll` z katalogu `lib/`. To ten plik '*.bc' bez sufiksu raw można odpalać za pomocą `lli`.

# Program kończy się kodem 1 w przypadku błędu analizy semantycznej (i wypisze błąd na STDERR) i kodem 0 w przypadku sukcesu. 
./latc_llvm tests/good/official/core001.lat
# lub
cargo run tests/good/official/core001.lat

# Można też dodać flagę `--run`, która uruchomi `lli` na skompilowanym pliku.
./latc_llvm tests/good/official/core001.lat --run
# lub
cargo run tests/good/official/core001.lat --run
```

### Reprezentacje pośrednie
- Domyślnie emitowanie wszystkich reprezentacji pośrednich jest wyłączone, po dodaniu odpowiednich flag, kompilator dodatkowo wypluje pliki z reprezentacjami pośrednimi (flagi można oczywiście ze sobą łączyć):
```bash
# Wyemituje plik ze sparsowanym AST (*.ast.json)
./latc_llvm tests/good/official/core001.lat --emit-ast
# Wyemituje plik z AST analize semantycznej i otypowaniu. (*.tast.json)
./latc_llvm tests/good/official/core001.lat --emit-tast
# Wyemituje plik z tekstową reprezentacja MIR (Mid Intermediate Representation). Prosta reprezentacja blokowa z kodem czwórkowym i bez złożonych LValue (zagnieżdżone access'y są już rozbite na kilka instrukcji). (*.mir)
./latc_llvm tests/good/official/core001.lat --emit-mir
# Wyemituje plik z tekstową reprezentacja LIR (Low Intermediate Representation). Reprezentacja blokowa z SSA i phi. (*.lir)
./latc_llvm tests/good/official/core001.lat --emit-lir
# Wyemituje plik z tekstową reprezentacja LIR otrzymaną po optymalizacjach (jeżeli takowe były włączone).
./latc_llvm tests/good/official/core001.lat --emit-lir-opt
# Wyemituje pliki ze wszystkimi reprezentacjami pośrednimi.
./latc_llvm tests/good/official/core001.lat --emit-all
```

### Optymalizacje
- Domyślnie wszystkie optymalizacje są wyłączone. Uruchamia się je zestawem flag:
```bash
./latc_llvm tests/good/official/core001.lat -O                # Włącza wszystkie optymalizacje.
./latc_llvm tests/good/official/core001.lat --opt-const-prop  # Constant propagation i constant folding.
./latc_llvm tests/good/official/core001.lat --opt-branch-fold # Branch folding.
./latc_llvm tests/good/official/core001.lat --opt-dbe         # Dead Block Elimination.
./latc_llvm tests/good/official/core001.lat --opt-dce         # Dead Code Elimination.
./latc_llvm tests/good/official/core001.lat --opt-max-iterations <OPT_MAX_ITERATIONS> # Opcjonalnie można ustawić maksymalną liczbe przejść przez program w czasie optymalizacji. Domyślnie maksymalna wartość to 10, ale optymalizator zatrzymuje się wcześniej jeżeli wykryje że wyoptymalizował już wszystko co się da.
```

## Struktura testów
- W przetestowaniu funkcjonalności może pomóc znajomość folderu z testami sprawdzić jak działa kompilator.
```bash
tests
├── bad                     # Testy, które powinny nie przejść analizy semantycznej i wypisać błąd
│   ├── lexical             # Test z błedami leksykalnymi
│   ├── official            # Oficjalne testy na przedmiot MRJP. Pomieszana błędny semantyczne, leksykalne, składniowe
│   ├── runtime             # Testy z błędami wykrywanymi w runtime'ie
│   └── semantic
│       ├── basic           # Dodatkowe testy latte bez rozszerzeń
│       ├── error_recovery  # Testy sprawdzające error recovery.
│       └── extensions      # Testy sprawdzające wykrywanie błędów semantycznych w przypadku rozszerzeń.
└─ good                     # Testy, które powinny przejść i powinien się dla nich wygenerować poprawny kod.
    ├── extensions          # Prawidłowe testy dla rozszerzeń, dla których powinien się generować poprawny kod.
    │   ├── arrays
    │   ├── objects
    │   ├── struct
    │   └── virtual
    ├── extra               # Dodatkowe testy podstawy latte, bez rozszerzeń.
    ├── official            # Oficjalne, prawidłowe testy MRJP
    └── opts                # Testy sprawdzające optymalizacje
        ├── branch_removal
        ├── constant_folding_and_propagation
        ├── dead_code_elimination
        ├── garbage_collector
        └── tail_call
```

## Struktura projektu
```bash
```md
src/
├── bin
│   └── latc_llvm.rs                # Punkt wejścia kompilatora. Uruchamia driver i pełen pipeline.
├── config.rs                       # Definicje opcji i argumentów CLI (Clap).
├── driver.rs                       # Główny sterownik kompilacji. Łączy: parser -> TAST -> MIR -> LIR -> OptLIR -> LLVM.
├── errors                          # Obsługa błędów i diagnostyka.
│   ├── diagnostics.rs              # Formatowanie błędów (Ariadne).
│   ├── lexical.rs                  # Błędy leksera.
│   ├── parsing.rs                  # Błędy parsera.
│   ├── semantic.rs                 # Błędy analizy semantycznej.
│   └── mod.rs
├── latte.lalrpop                   # Plik gramatyki LALRPOP.
├── lib.rs                      
├── mir                             # MIR – blokowa reprezentacja pośrednia.
│   ├── builders                    # Buildery do budowania grafu bloków MIR.
│   ├── lowering                    # Obniżanie TAST (Typed AST) do MIR.
│   ├── mir_structure.rs            # Struktury danych dla MIR (instrukcje, bloki).
│   ├── mir_display.rs              # Debug-print dla MIR.
│   └── mod.rs
├── lir                             # LIR – Low Intermediate Representation (z SSA).
│   ├── lowering                    # Transformacja MIR -> LIR.
│   │   ├── cfg.rs                  # Analiza grafu przepływu sterowania, dominatory itp.
│   │   ├── function_lowering.rs    # Generowanie postaci SSA (wstawianie węzłów Phi).
│   │   ├── program_lowering.rs     # Koordynacja obniżania całego programu.
│   │   └── mod.rs
│   ├── lir_structure.rs            # Definicje rejestrów wirtualnych, instrukcji SSA i terminatorów.
│   ├── lir_display.rs              # Formatowanie LIR do czytelnego formatu tekstowego.
│   └── mod.rs
├── llvm                            # Backend LLVM.
│   ├── codegen.rs                  # Główny generator kodu LLVM IR z LIR
│   ├── driver.rs                   # Integracja z zewnętrznymi narzędziami (llvm-as, llvm-link, lli).
│   └── mod.rs
├── opts                            # Moduł optymalizacji kodu, działający na LIR przed generacja kodu LLVM.
│   ├── branch_folding.rs           # Usuwanie niepotrzebnych gałęzi.
│   ├── constant_propagation.rs     # Propagacja stałych i zwijanie stałych.
│   ├── dead_block_elimination.rs   # Usuwanie martwych bloków.
│   ├── dead_code_elimination.rs    # Usuwanie martwego kodu.
│   ├── mod.rs                      # Główna logika uruchamiania optymalizacji na LIRze.
│   ├── pass.rs                     # Trait specyfikujący pojedynczą optymalizacje.
│   └── phi_removal.rs              # Usuwanie uproszczonych phi (jeszcze nie zaimplementowane).
├── parser                          # Lekser, parser i surowe AST.
│   ├── ast                         # Węzły AST (nietypowane).
│   ├── lexer.rs                    # Lekser (Logos).
│   ├── tokens.rs                   # Definicje tokenów.
│   └── mod.rs
├── semantics                       # Analiza semantyczna i generacja TAST.
│   ├── analysis
│   │   ├── context.rs              # Kontekst analizy (scope, tablica symboli).
│   │   ├── definition_checker.rs   # Weryfikacja definicji (funkcji/klas)
│   │   ├── expr_checker.rs         # Sprawdzanie typów wyrażeń.
│   │   ├── stmt_checker.rs         # Sprawdzanie instrukcji.
│   │   ├── return_checker.rs       # Weryfikacja return.
│   │   ├── type_checker.rs         # Logika zgodności typów.
│   │   └── mod.rs
│   ├── builtins.rs                 # Definicje funkcji wbudowanych (printInt, printString, itp.).
│   ├── symbol_table.rs             # Tablica symboli (zmienne, funkcje, klasy).
│   ├── symbols.rs                  # Struktury pomocnicze dla symboli.
│   ├── tast                        # TAST — otypowane AST (wynik analizy semantycznej).
│   └── mod.rs
└── utils
    ├── str_id.rs                   # String interning (StrID/Ustr).
    └── mod.rs
```

## Użyte biblioteki (Crates)
*   [**Clap**](https://crates.io/crates/clap) – parsowanie argumentów linii poleceń.
*   [**Lalrpop**](https://github.com/lalrpop/lalrpop) – generator parsera.
*   [**Logos**](https://crates.io/crates/logos) – generator leksera.
*   [**Ariadne**](https://crates.io/crates/ariadne) – biblioteka do wizualizacji błędów kompilacji (kolorowanie składni, podkreślanie błędów).
*   [**Ustr**](https://crates.io/crates/ustr) – string interning (używane jako `StrID`), co przyspiesza porównywanie identyfikatorów i oszczędza kopiowania stringów.
*   [**Serde**](https://crates.io/crates/serde) – serializacja struktur danych.

# Latte - Piotr Oleszczuk, nr 448390
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
      - `make clean` - usuwa zbudowane binarki oraz pliki `*.{mir.txt, tast.txt, json, ll, bc}` z katalogu `tests/in`
      - `make clean-all` - usuwa to co `make clean` oraz wszystkie pliki dependencji

## Raport z pierwszej iteracji
- W czasie pisania projektu, trochę zmieniały mi się plany na implementacje. Początkowo planowałem przejść 
  przez cały pipeline kompilatora wąsko (z małą liczbą feature'ów) i nastepie po kolei dodawać do niego 
  kolejne funkcjonalności. Ostatecznie plan się zmienił i przez warstwy przechodziłem grubo (od razu ze wszystkimi feature'ami).
- Aktualny stan projektu jest taki, że działa pełna analiza semantyczna dla języka ze wszystkimi rozszerzeniami(tablice, struktury, 
  klasy, dziedziczenie), która jest zintegrowana z Ariadne(błędy są naprawde ładne i duo czasu zajęło mi pisanie ich (pewnie 
  nawet za dużo, zakładam że troche zoverkillowałem)). Działa także obniżanie reprezentacji drzewowej do MIRa(Mid Intermediate 
  Representation, reprezentacja blokowa programu, jeszcze bez SSA(single static assignment)) dla wszystkich feature'ów.
- Ostatecznie powstało na razie 5600 linii kodu

### Co miało być wykonane i zostało wykonane
- W pełni funkcjonalny parser, z ładnymi błędami parsowania - zrobione, nawet z analizą semantyczną dla rozszerzeń
- Integracja z lalrpopem, ariadne, logosem - zrobione, lekser używa logosa, parser lalrpopa, do błędów używam ariadne
- Zaimplementowana semantyczna weryfikacja dla podstawowej wersji(+ stringi) - zrobione nie tylko dla wersji podstawowej, jest też analiza semantyczna dla wszystkich rozszerzeń
- Asynchroniczny "Framework" do testów kompilatora zintegrowany z cargo - nie ukrywam, że to nie jest do końca to co planowałem, ale cargo automatycznie puszcza testy współbieżnie, więc jakby przymknąć oko to współbieżne testy kompilatora są :). Tutaj jestem całkiem zadowolony z automatycznej generacji testów (`build.rs`) dla plików w folderach testowych, które tworzą się automatycznie, gdy nowy plik testowy pojawi się w folderze. Jestem ciekawy na ile takie rozwiązanie jest "produkcyjne".
    - Testy parsowania - zrobione
    - Testy poprawności semantycznej - zrobione
    - Podbudowa pod testy integracyjne - powiem szczerze, że aktualny stan implementacji tej części to nie jest to czego się spodziewałem 
        na oddając plan projektu, ale myślę, że wykonaną pracę można uznać za "podbudowe", więc uznam ten punkt za zrobiony.
### Co miało być wykonane i nie zostało wykonane
- Zaimplementowana prosta logika ewaluacji wyrażeń podczas kompilacji - tego nie ma, stwierdziłem, że to optymalizacja czym wole zająć się 
  potem (albo optymalizować LIRa(low intermediate representation, reprezentacja już z SSA) a nie AST). Priotytetyzowałem przepchnięcie się przez cały pipeline kompilacji
### Co nie miało być wykonane i zostało wykonane
- Analiza semantyczna dla wszystkich rozszerzeń
- Solidne error recovery - mój kompilator nie wywala się na pierwszym błędzie, tylko całkiem mądrze handluje i robi recovery jeżeli będzie błąd. Kompilacja programu nie wywala się na pierwszym spotkanym błędzie tylko zwraca listę błędów.
- Dodanie obniżania aż do MIRa(czyli jakies dodatkowe 2k linii kodu)
- String interning i StrID używane w całym projekcie (`src/utils/StrID`)

## Użycie kompilatora
```bash
# Wykona parsowanie, analize semantyczną oraz przejście przez dwie pierwsze fazy kompilacji, generacje otypowanego AST oraz MIRa.
# Wszystkie reprezentacje pośrednie zostaną zapisane w plikach (domyślnie w katalogu, w którym znajduje się plik kompilowany, można też dodać flage --output-dir)
# plik: {nazwa_testu}.ast.json - zawiera zserializowane AST, bezpośrednio po parsowaniu pliku
# plik: {nazwa_testu}.tast.json - zawiera zserializowane AST, wzbogacone o informacje typową.
# plik: {nazwa_testu}.mir.json - zawiera zserializowanego MIRa (raczej nie ma się tam co patrzeć, wygodniejszy do oglądania jest *.mir.txt)
# plik: {nazwa_testu}.mir.txt - zawiera zserializowanego MIRa, co własnego formatu przyjemnego dla oka i możliwego do czytania.

# Program kończy się kodem 1 w przypadku błędu analizy semantycznej (i wypisze błąd na STDOUT) i kodem 0 w przypadku sukcesu. 
./latc_llvm tests/in/test01.ins 
# lub
cargo run tests/in/test01.ins 
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
└─ good                     # Testy, które powinny przejść i powinien się dla nich wygenerować poprawny MIR.
    ├── extensions          # Prawidłowe testy dla rozszerzeń, dla których powinien się generować poprawny MIR.
    │   ├── arrays
    │   ├── objects
    │   ├── struct
    │   └── virtual
    ├── extra               # Dodatkowe testy podstawy latte, bez rozszerzeń.
    └── official            # Oficjalne, prawidłowe testy MRJP


```


## Struktura projektu
```bash
```md
src/
├── bin
│   └── latc_llvm.rs                # Punkt wejścia kompilatora. Uruchamia driver i pełen pipeline.
├── config.rs                       # Definicje opcji i argumentów CLI (Clap).
├── driver.rs                       # Główny sterownik kompilacji. Łączy: parser -> TAST -> MIR -> LIR -> LLVM.
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
```
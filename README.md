# Velora v11 🚀

Linguaggio di programmazione evolutivo con AI-first design e self-hosting.

## ✨ Features

- **AI-First**: Generazione codice via GROK, OpenAI, Claude, Ollama
- **Self-Hosting v11**: Velora genera codice per se stessa (`# AI: self ...`)
- **Package Manager**: `vel add`, `vel install`, registry GitHub
- **LSP Server**: Autocompletion, go-to-definition
- **Testing**: `test "nome" { assert_eq(...) }`
- **Errori UX**: Stack trace, suggestions, colori

## 🚀 Installazione

```bash
cd velora-v3
cargo build --release
```

## 📦 Package Manager

```bash
# Installa lo script vel
cp ~/bin/vel /usr/local/bin/vel  # oppure lascia in ~/bin

# Comandi
vel init my-project              # Nuovo progetto
vel add math-lib                 # Installa dipendenza
vel install                      # Installa da velora.toml
vel run main.vel                 # Esegue file
```

## 🧬 Self-Hosting

```velora
# AI: self aggiungi supporto per quantum computing

main:
    print(auto_evolved_feature())
```

Output:
```
🤖 SELF-HOSTING v11 attivato
Quantum bit attivato: 0|1
```

## 🧪 Testing

```velora
fn sum(a: Int, b: Int) -> Int {
    return a + b
}

test "sum works" {
    assert_eq(sum(2, 3), 5)
    assert_true(sum(0, 0) == 0)
}

main:
    print(sum(10, 20))
```

```bash
velora test file.vel
```

## 📝 Syntax

```velora
# Importa moduli
import "utils"
import "math" as m

# Funzioni
fn square(x: Float) -> Float {
    return x * x
}

# Types: Int, Float, String, Bool, Array, Map, Option, Result
fn divide(a: Float, b: Float) -> Result {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

main:
    # Variabili
    let x = 42
    let pi = 3.14159
    let name = "Velora"
    
    # Array
    let nums = [1, 2, 3, 4, 5]
    print(nums[0])           # 1
    print(len(nums))         # 5
    
    # Map
    let user = {name: "Mario", age: 30}
    print(map_get(user, "name"))
    
    # Option/Result
    let res = divide(10.0, 2.0)
    if is_ok(res) {
        print(unwrap(res))   # 5.0
    }
    
    # While
    let i = 0
    while i < 3 {
        print(i)
        i = i + 1
    }
    
    # If/else
    if x > 10 {
        print("grande")
    } else {
        print("piccolo")
    }
```

## 🔧 Built-in Functions

| Categoria | Funzioni |
|-----------|----------|
| Numeriche | `fibonacci`, `abs`, `sqrt`, `pow`, `floor`, `ceil`, `round` |
| Array | `len`, `push`, `contains` |
| Stringhe | `split`, `trim`, `starts_with`, `ends_with`, `to_upper`, `to_lower`, `substring`, `replace` |
| Map | `map_get`, `map_set`, `map_keys`, `map_has` |
| Option/Result | `Some`, `None`, `Ok`, `Err`, `unwrap`, `unwrap_or`, `is_ok`, `is_err`, `is_some`, `is_none` |
| File | `read_file`, `write_file`, `append_file`, `file_exists` |
| Test | `assert_eq`, `assert_true`, `assert_false` |

## 🏗️ Architettura

```
velora-v3/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── parser.rs         # Pest parser + error UX
│   ├── interpreter.rs    # Interpreter + built-in
│   ├── ast.rs            # AST definitions
│   ├── error.rs          # Pretty errors + suggestions
│   ├── ai.rs             # AI integration + self_host()
│   ├── package.rs        # Package manager
│   ├── lsp.rs            # LSP server
│   └── ai/               # AI providers (GROK, Ollama, etc.)
├── velora-packages/      # Package registry
└── Cargo.toml
```

## 🤖 AI Providers

Configura le API keys:
```bash
export GROK_API_KEY="xai-..."
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export OLLAMA_HOST="http://localhost:11434"
```

## 📄 License

MIT

---

**Velora v11** - Il linguaggio che si evolve da solo 🧬

#!/bin/bash
# Test veloce con nemotron usando few-shot prompting

echo "🧪 Test Nemotron-mini-4b con few-shot prompting"
echo ""

# Test diretto con Ollama usando few-shot
ollama run nemotron-mini-4b << 'EOF'
Generate Velora code. ONLY code, no explanations.

Example 1:
fn double(x) -> Int {
    return x * 2
}
print(double(5))

Example 2:
fn sum(a, b) -> Int {
    return a + b
}
print(sum(3, 4))

Now generate: fn factorial(n) -> Int
EOF

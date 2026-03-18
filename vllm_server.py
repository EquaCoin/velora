#!/usr/bin/env python3
"""
vLLM Inference Server per Velora
Ottimizzato per NVIDIA DGX Spark (GB10 Blackwell)
"""

import sys
import json
import argparse
from pathlib import Path

def check_vllm():
    """Verifica che vLLM sia installato"""
    try:
        import vllm
        print(f"✅ vLLM {vllm.__version__} trovato")
        return True
    except ImportError:
        print("❌ vLLM non installato. Installa con: pip3 install vllm")
        return False

def generate_code(prompt: str, model: str = "nemotron-30b") -> str:
    """Genera codice Velora usando vLLM"""
    
    from vllm import LLM, SamplingParams
    
    system_prompt = """You are a code generator for Velora programming language.
Generate ONLY valid Velora code. No explanations, no markdown, no comments.

Velora syntax:
- Variables: let x = 10
- Print: print(x)
- Functions: fn name(x) -> Int { return x * 2 }
- If: if x > 5 { print("big") } else { print("small") }
- While: while x > 0 { print(x); let x = x - 1 }

EXAMPLE:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
print(factorial(5))

Generate code for:"""

    full_prompt = f"{system_prompt} {prompt}\n\n"
    
    print(f"🔄 Caricamento modello {model}...")
    
    # Configura vLLM per DGX Spark
    llm = LLM(
        model=model,
        tensor_parallel_size=1,  # Single GPU
        gpu_memory_utilization=0.95,
        max_model_len=4096,
        dtype="auto",  # Auto-detect FP16/BF16
    )
    
    sampling_params = SamplingParams(
        temperature=0.3,
        max_tokens=500,
        stop=["###", "<|endoftext|>", "<|im_end|>"],
    )
    
    print(f"🤖 Generazione codice...")
    outputs = llm.generate(full_prompt, sampling_params)
    
    generated_text = outputs[0].outputs[0].text
    return extract_code(generated_text)

def extract_code(text: str) -> str:
    """Estrae solo il codice dalla risposta"""
    text = text.strip()
    
    # Se c'è un code block
    if "```" in text:
        start = text.find("```")
        after_start = text[start + 3:]
        code_start = after_start.find('\n')
        after_lang = after_start[code_start:]
        
        if "```" in after_lang:
            end = after_lang.find("```")
            return after_lang[:end].strip()
    
    # Filtra righe valide
    valid_starts = ["fn ", "main:", "let ", "print(", "if ", "while ", "return"]
    lines = []
    
    for line in text.split('\n'):
        trimmed = line.strip()
        if any(trimmed.startswith(s) for s in valid_starts) and len(trimmed) < 100:
            lines.append(trimmed)
    
    return '\n'.join(lines)

def main():
    parser = argparse.ArgumentParser(description="vLLM Server per Velora")
    parser.add_argument("--model", default="nemotron-30b", help="Modello da usare")
    parser.add_argument("--prompt", required=True, help="Richiesta di generazione")
    parser.add_argument("--check", action="store_true", help="Verifica installazione")
    
    args = parser.parse_args()
    
    if args.check:
        if check_vllm():
            print("✅ Sistema pronto per vLLM")
            sys.exit(0)
        else:
            sys.exit(1)
    
    if not check_vllm():
        sys.exit(1)
    
    try:
        code = generate_code(args.prompt, args.model)
        print("\n" + "="*50)
        print("CODICE GENERATO:")
        print("="*50)
        print(code)
        print("="*50)
    except Exception as e:
        print(f"❌ Errore: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()

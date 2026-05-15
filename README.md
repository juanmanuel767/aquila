<p align="center">
  <img src="https://raw.githubusercontent.com/juanmanuel767/aquila/main/website/assets/logo.png" width="300" alt="NeuroCode Logo">
</p>

# NeuroCode: El Lenguaje IA-Nativo en Español

<p align="center">
  <img src="https://img.shields.io/badge/Versi%C3%B3n-v1.0.0--estable-3b82f6?style=for-the-badge" alt="Version">
  <img src="https://img.shields.io/badge/N%C3%BAcleo-Rust-black?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Licencia-MIT-green?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/IA-Nativa-f3c75f?style=for-the-badge" alt="AI Native">
</p>

---

**NeuroCode**  es un lenguaje de programación de alto rendimiento construido en **Rust**, diseñado para automatizar procesos, integrar Inteligencia Artificial de forma nativa y permitir el desarrollo de software complejo utilizando una sintaxis expresiva en **español**.

## ✨ Características Principales

- 🧠 **IA Nativa**: Consulta LLMs (Ollama, Anthropic, etc.) directamente con la función `ia()`.
- 🏹 **Modo Cazador**: Importa y usa cualquier librería de Python de forma transparente.
- ⚡ **Alto Rendimiento**: Motor de ejecución escrito en Rust, rápido y seguro.
- 🛡️ **Autocuración**: El "Guardián" integrado analiza y repara errores de código automáticamente usando IA.
- 🌐 **Servidor Web & Async**: Crea APIs web con un runtime asíncrono moderno.

---

## 🚀 Despegue Rápido

### Requisitos
- [Rust](https://rustup.rs/) (v1.75 o superior)

### Instalación desde Código Fuente
1. Clona el repositorio:
   ```bash
   git clone https://github.com/tu-usuario/neurocode.git
   cd neurocode
   ```
2. Compila el intérprete:
   ```bash
   cargo build --release
   ```
3. (Opcional) Mueve el binario a tu path o úsalo directamente:
   ```bash
   ./target/release/neurocode mi_script.neuro
   ```

---

## 💻 Código de Ejemplo

Así de simple es crear lógica potente en NeuroCode:

```aquila
// Un script que usa lógica y IA
funcion saludar(nombre) {
    retornar "Hola, " + nombre + ". ¿En qué puedo ayudarte?"
}

imprimir(saludar("Desarrollador"))

intentar {
    idea = ia("Dame una idea breve para un agente de automatización")
    imprimir("Sugerencia de la IA:", idea)
} capturar error {
    imprimir("No pude conectar con el cerebro:", error)
}
```

---

## 📚 Documentación

- [📖 Guía de Sintaxis](docs/guia-sintaxis.md): Aprende a programar en NeuroCode.
- [📂 Ejemplos de Integración](docs/ejemplos/):
    - [FizzBuzz](docs/ejemplos/fizzbuzz.neuro)
    - [Fibonacci](docs/ejemplos/fibonacci.neuro)
    - [Encontrar el Mayor](docs/ejemplos/mayor.neuro)

---

## 👤 El Visionario
**Juan Manuel Peralta**  
*Arquitecto Maestro de NeuroCode*

---
<p align="center">Forjado en Rust · IA Nativa · Python FFI. © 2026 NeuroCode Labs.</p>

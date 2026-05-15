<p align="center">
  <img src="https://raw.githubusercontent.com/juanmanuel767/Neuro-code/main/website/assets/logo.png" width="300" alt="NeuroCode Logo">
</p>

# NeuroCode: El Lenguaje IA-Nativo en Español

<p align="center">
  <img src="https://img.shields.io/badge/Versi%C3%B3n-v2.3.0--estable-3b82f6?style=for-the-badge" alt="Version">
  <img src="https://img.shields.io/badge/N%C3%BAcleo-Rust-black?style=for-the-badge&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/Licencia-MIT-green?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/IA-Nativa-f3c75f?style=for-the-badge" alt="AI Native">
</p>

---

**NeuroCode**  es un lenguaje de programación de alto rendimiento construido en **Rust**, diseñado para automatizar procesos, integrar Inteligencia Artificial de forma nativa y permitir el desarrollo de software complejo utilizando una sintaxis expresiva en **español**.

- 🧠 **IA Nativa**: Consulta LLMs (Ollama, Anthropic, etc.) directamente con la función `ia()`.
- ⚡ **Reactividad Nativa**: Sistema de observadores con `reactivo` y `cuando...cambie`.
- 🧬 **Herencia de Clases**: Programación orientada a objetos avanzada con `hereda` y `super()`.
- 📦 **Modularidad**: Gestión de módulos limpia con `importar` y `exportar`.
- 🏹 **Modo Depredador**: Importa y usa cualquier librería de Python de forma transparente.
- 🛡️ **Autocuración**: El "Guardián" integrado analiza y repara errores automáticamente.

---

## 🚀 Despegue Rápido

### Requisitos
- [Rust](https://rustup.rs/) (v1.75 o superior)

### Instalación desde Código Fuente
1. Clona el repositorio:
   ```bash
   git clone https://github.com/juanmanuel767/Neuro-code.git
   cd Neuro-code
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

```neurocode
// 1. Reactividad Nativa
reactivo precio = 100
cuando precio cambie {
    imprimir(">>> Alerta: El precio subió a", precio)
}
precio = 150 // Dispara la alerta automáticamente

// 2. Herencia y IA
clase IA_Agente hereda ServidorWeb {
    crear(puerto) {
        super(puerto)
    }
    
    asincrono funcion pensar(prompt) {
        idea = esperar ia(prompt)
        imprimir("🧠", idea)
    }
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

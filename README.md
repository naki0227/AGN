# AGN (Antigravity-Native) 🚀

<p align="center">
  <b>"AI becomes the Intelligence, GPU becomes the Wings."</b><br>
  The world's first AI-native, multilingual programming language powered by Rust and GPU.
</p>

---

[🇯🇵 Japanese (日本語)](README.ja.md)

## 🌟 What is AGN?

**AGN (Antigravity-Native)** is a next-generation programming language that fuses the intuitiveness of natural language (Japanese/English) with the overwhelming performance of low-level systems (Rust/LLVM).

Designed for the Google Antigravity environment, the AI agent understands your "intent" and optimizes the code.

### 💎 Key Features

*   **Multilingual Native Syntax**
    *   Treats Japanese (SOV) and English (SVO) equivalently, converting them into the same logical structure (Unified AST).
    *   *JP:* `X を 並列で 表示する`
    *   *EN:* `parallel show X`

*   **AI as a First-class Citizen**
    *   AI inference is embedded as a standard "verb".
    *   `Summarize input into result`

*   **GPU-Accelerated Universal UI**
    *   Uses `wgpu` backend to render Dribbble/Rive-class rich UIs at 60fps.

*   **Antigravity-Speed**
    *   Generates native binaries comparable to Rust/C++ via LLVM IR output.

---

## 🚀 Quick Start

### 📋 Prerequisites
*   Rust (latest stable)
*   LLVM 15+
*   Google Antigravity IDE (Recommended)

### 🛠 Installation & Run
```bash
git clone https://github.com/naki0227/AGN.git
cd AGN
cargo run -- examples/demo_phase12.agn --run-compiled
```

## 🗺 Roadmap
- [x] **Phase 1-3:** Core Kernel & LLVM Backend
- [x] **Phase 4-6:** Multilingual SVO & Universal UI (Wasm/Native)
- [x] **Phase 7-12:** GPU Rendering (wgpu) & Interactive Animations
- [ ] **Phase 13:** Mobile Native Support (iOS/Android)
- [ ] **Phase 14:** AI-driven Auto-Refactoring

## 🤝 Contribution
AGN is an open-source project aiming to be the "World's Best". Bug reports, feature proposals, and pull requests are welcome!

**Developer**: naki0227  
**Portfolio**: https://enludus.vercel.app

## 📄 License
This project is licensed under the MIT License.

# Contributing to wasmcp

Thanks for your interest in contributing!

## Ways to Contribute
- **Report bugs** or request features via [GitHub Issues](../../issues).  
- **Improve documentation** (README, examples, tutorials).  
- **Submit patches** via Pull Requests (PRs).  
- **Propose design changes** via the RFC process.

## Development Setup
1. Clone the repo.  
2. Install Rust (latest stable) and `cargo-component`.  
3. Install [wac](https://github.com/bytecodealliance/wac) and [wkg](https://github.com/bytecodealliance/wkg).  
4. Run `cargo component build` to build components.  
5. Run tests with `cargo test`.

## Pull Requests
- Small fixes: open a PR directly.  
- Larger changes (new APIs, breaking WIT changes): open an issue first to discuss
  or submit an RFC.  
- All PRs require review by at least one maintainer.  
- Keep commits focused; rebase if necessary before merging.

## RFC Process
For substantial changes (e.g. protocol semantics, new transports), create a PR
adding a markdown doc under `/rfcs`.  
The RFC should describe the motivation, design, and alternatives considered.  
After discussion and approval, the RFC is merged and guides implementation.

## Code of Conduct
This project follows the [Contributor Covenant](https://www.contributor-covenant.org/).
All contributors are expected to uphold it.

## Developer Certificate of Origin (DCO)
By contributing, you agree to the [DCO](https://developercertificate.org/).
Sign your commits with `git commit -s`.

## Security Issues
Please **do not open a public issue** for security problems.  
Instead, email **security@wasmcp.org** (or appropriate contact).  
We will coordinate a fix and disclosure.

---

We welcome your contributions and feedback!

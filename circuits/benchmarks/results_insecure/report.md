# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-27 10:35:23 UTC

**Git Branch:** `main`  
**Git Commit:** `e651f30205117e83a9cc48ac44664d434466724d`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 25.76            | 15.88           |
| C1                   | 57818       | 0.33           | 26.00            | 15.88           |
| C2a                  | 142625      | 0.77           | 26.58            | 15.88           |
| C2b                  | 198355      | 0.86           | 26.30            | 15.88           |
| C3a                  | 132633      | 0.79           | 25.93            | 15.88           |
| C3b                  | 132633      | 0.79           | 25.93            | 15.88           |
| C4a                  | 92515       | 0.49           | 25.95            | 15.88           |
| C4b                  | 92515       | 0.49           | 25.95            | 15.88           |
| C5                   | 151717      | 0.79           | 25.87            | 15.88           |
| user_data_encryption | 53732       | 0.32           | 25.68            | 15.88           |
| C6                   | 86927       | 0.52           | 26.68            | 15.88           |
| C7                   | 104273      | 0.49           | 26.20            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 15.88 KB   | 0.12 KB           | 3037849    | 179452       | 3217301   |
| Π_user   | 31.75 KB   | 0.22 KB           | 2972953    | 340140       | 3313093   |
| Π_dec    | 15.88 KB   | 3.25 KB           | 3549211    | 188232       | 3737443   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 3.37 s     | 95.25 KB   | 96.12 KB  |
| Aggregator      | P2    | combine folds + C5               | 0.79 s     | 15.88 KB   | 16.00 KB  |
| User            | P3    | per user input                   | 0.64 s     | 31.75 KB   | 31.97 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.52 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 0.49 s     | 15.88 KB   | 19.12 KB  |

## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 14
- **RAM:** 48.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.16 noirc version =
  1.0.0-beta.16+2d46fca7203545cbbfb31a0d0328de6c10a8db95 (git version hash:
  2d46fca7203545cbbfb31a0d0328de6c10a8db95, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20260102

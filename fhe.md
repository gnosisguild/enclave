Fully Homomorphic Encryption — From Zero to Hero

A complete lecture series covering every concept you need, built from the ground up.

---

Lecture 1: What Problem Are We Solving?

The Dream

Imagine you have medical records. You want a cloud server to run analytics on them — but you don't
trust the server. You want to encrypt the data, send the encrypted data to the server, have the
server compute on the encrypted data, and get back an encrypted result that only you can decrypt.

This is Fully Homomorphic Encryption (FHE).

You: encrypt(x) ──────► Server: f(encrypt(x)) ──────► You: decrypt(result) = f(x) ↑ Server never
sees x!

The server computes f on ciphertext and gets the same answer as if it computed f on plaintext. It
never learns what x is.

Why Is This Hard?

Normal encryption (like AES) scrambles data so thoroughly that you can't do math on it. FHE needs
encryption that is:

1. Secure — can't break it
2. Structured — addition and multiplication "pass through" the encryption

The trick: noise-based encryption. We hide the message behind random noise. But every operation
makes the noise grow. Eventually the noise overwhelms the message and decryption fails. The central
challenge of FHE is managing noise.

---

Lecture 2: Modular Arithmetic & The Torus

Modular Arithmetic (Clock Math)

On a 12-hour clock, 10 + 5 = 3 (we wrap around). This is arithmetic modulo 12, written 10 + 5 ≡ 3
(mod 12).

More generally, a mod q means "the remainder when dividing a by q". All arithmetic happens in {0, 1,
..., q-1}.

The Torus T = R/Z

The Torus is like a clock, but continuous. Take the real number line and wrap it so that every
integer maps to the same point. What remains is just the fractional part of any real number:

T = R/Z = the set of real numbers modulo 1

Examples: 3.7 → 0.7 -0.3 → 0.7 (same point!) 1.0 → 0.0 0.25 → 0.25

Think of it as a circle of circumference 1. Addition wraps around naturally: 0.8 + 0.4 = 0.2 (on the
Torus).

Why the Torus?

The Torus is the natural home for TFHE (Torus FHE). Messages are encoded as positions on the circle,
and noise is a small random jitter around that position. As long as the jitter is small enough, we
can round back to the correct position.

Encoding 1 bit:

       0.0
        |
    0 ──●── 1      0 maps to position 0.0
        |           1 maps to position 0.5
       0.5          Noise moves us slightly off these points
                    Decryption: round to nearest {0.0, 0.5}

Fixed-Point Representation

Computers can't store real numbers exactly. We approximate Torus elements using fixed-point
integers. With k bits of precision:

Torus element t ≈ round(t × 2^k) stored as a signed integer

Example (k = 8 bits): t = 0.25 → 0.25 × 256 = 64 → stored as 64 t = 0.75 → 0.75 ×

---

Lecture 3: Polynomials & Polynomial Rings

Polynomials

A polynomial is just a list of coefficients:

p(X) = 3 + 2X + 5X² + X³

Coefficients: [3, 2, 5, 1] Degree: 3

Adding polynomials: add corresponding coefficients. Multiplying polynomials: like multiplying
multi-digit numbers, but with powers of X instead of powers of 10.

Polynomial Rings: Z[X]/(X^N + 1)

We work in a special ring where:

- Coefficients are integers (Z)
- After every operation, we reduce modulo X^N + 1

What does "modulo X^N + 1" mean? It means X^N = -1. So any power X^{N+k} wraps around:

X^N = -1 X^{N+1} = -X X^{N+2} = -X² ...and so on

This keeps all polynomials at degree < N. With N = 4 as a toy example:

(1 + X²) × (X + X³) = X + X³ + X³ + X⁵ = X + 2X³ + X⁵ = X + 2X³ + X · X⁴ = X + 2X³ + X · (-1) [since
X⁴ = -1] = 2X³

Why This Ring?

The ring Z[X]/(X^N + 1) (called a cyclotomic ring) has two crucial properties:

1. Security: The Ring-LWE problem in this ring is believed to be as hard as worst-case lattice
   problems. Quantum computers can't efficiently solve it (as far as we know).
2. Efficiency: Polynomial multiplication can be done in O(N log N) using the Number Theoretic
   Transform (NTT) — a variant of the Fast Fourier Transform. Without this, multiplication would be
   O(N²).

---

Lecture 4: LWE — Learning With Errors

The LWE Problem

LWE (Learning With Errors) is the mathematical hardness assumption that makes FHE secure.

Setup: Fix a secret vector s = (s₁, s₂, ..., sₙ) with small integer entries.

Encryption of a message m:

1. Pick a random vector a = (a₁, a₂, ..., aₙ)
2. Compute b = ⟨a, s⟩ + m + e where e is a small random error
3. Ciphertext = (a, b)

⟨a, s⟩ means the dot product: a₁s₁ + a₂s₂ + ... + aₙsₙ

Decryption (knowing s): b - ⟨a, s⟩ = m + e ≈ m (round away the small error)

Security: Without knowing s, the pair (a, b) looks completely random. The error e is what makes this
hard — without it, you could solve a system of linear equations to find s.

Why Is LWE Hard?

Given many samples (aᵢ, bᵢ), distinguishing them from truly random pairs is (believed to be)
computationally intractable. This is related to hard problems on lattices — geometric structures in
high-dimensional space.

Think of it like this: you're given noisy equations and asked to find the secret. The noise makes it
impossible to use Gaussian elimination or any efficient linear algebra technique.

Noise: The Double-Edged Sword

- Too little noise: encryption is insecure (solvable by linear algebra)
- Too much noise: decryption fails (can't round to correct message)
- Just right: secure AND decryptable

Every homomorphic operation increases the noise. This is the fundamental tension of FHE.

---

Lecture 5: Ring-LWE (RLWE) and GLWE

From Vectors to Polynomials

LWE works with vectors of integers. Ring-LWE replaces vectors with polynomials in Z[X]/(X^N + 1).
This gives us:

- Compact ciphertexts: one polynomial of degree N replaces N individual integers
- Fast operations: polynomial multiplication via NTT is O(N log N)
- SIMD-like packing: N message slots in one ciphertext

GLWE: Generalized (Ring-)LWE

A GLWE ciphertext with rank k consists of k+1 polynomials:

ct = (c₀, c₁, c₂, ..., cₖ)

where: c₁, ..., cₖ are random mask polynomials c₀ = body = -∑(cᵢ · sᵢ) + m + e

    sᵢ are secret key polynomials
    m is the plaintext polynomial
    e is a small error polynomial

Decryption: m + e = c₀ + ∑(cᵢ · sᵢ) = c₀ + c₁·s₁ + c₂·s₂ + ... + cₖ·sₖ

Special cases:

- rank = 0: trivial encryption (no mask, just ct = (m + e))
- rank = 1: standard RLWE (most common — one mask polynomial)
- rank > 1: generalized form (more mask polynomials, different noise/security trade-offs)
- N = 1 (degree-0 polynomials = scalars): plain LWE

The Encryption/Decryption Dance

Let's trace through a concrete example with rank = 1, N = 4:

Secret key: s₁(X) = 1 + X - X³ (small coefficients)

ENCRYPT message m(X) = 5 + 3X²: 1. Sample random mask: c₁(X) = 7 - 2X + X² + 4X³ 2. Sample small
error: e(X) = 0 + 1 - 1 + 0 (tiny!) 3. Compute body: c₀ = -c₁ · s₁ + m + e (polynomial
multiplication mod X⁴+1, then add message and error) 4. Ciphertext: ct = (c₀, c₁)

DECRYPT (knowing s₁): 1. Compute: c₀ + c₁ · s₁ = m + e 2. Round away e → m = 5 + 3X² ✓

---

Lecture 6: The Torus Representation in Detail

Base-2^K Decomposition (Limbs)

Here's where poulpy gets clever. Instead of storing each Torus coefficient as a single large
integer, poulpy decomposes it into limbs (digits in base 2^K):

coefficient c = c₀ + c₁·2^{-K} + c₂·2^{-2K} + ... + c\_{d-1}·2^{-(d-1)K}

Think of this like writing a number in a particular base: Decimal 1234 = 1×10³ + 2×10² + 3×10¹ +
4×10⁰ (base 10, 4 digits) Binary 1011 = 1×2³ + 0×2² + 1×2¹ + 1×2⁰ (base 2, 4 digits) Base-2^K: c₀ +
c₁·2^{-K} + ... (base 2^K, d limbs)

Parameters:

- base2k (K) — bits per limb (e.g., K = 17 means each limb holds a 17-bit value)
- size (d) — number of limbs
- Total precision = d × K bits

The Bivariate View: Z[X, Y]

Here's the mathematical elegance. If we treat the limb index as a second variable Y = 2^{-K}, then a
GLWE coefficient becomes a bivariate polynomial:

c(X, Y) = c₀(X) + c₁(X)·Y + c₂(X)·Y² + ... + c\_{d-1}(X)·Y^{d-1}

where each cⱼ(X) is a polynomial in X of degree < N

So a single GLWE polynomial is really a 2D array: Limb 0 Limb 1 Limb 2 ... Coeff X⁰: c₀[0] c₁[0]
c₂[0] Coeff X¹: c₀[1] c₁[1] c₂[1] Coeff X²: c₀[2] c₁[2] c₂[2] ... Coeff X^{N-1}: c₀[N-1] c₁[N-1]
c₂[N-1]

Why Limbs? — Normalization and Rescaling

Normalization is the process of propagating carries between limbs, just like carrying in decimal
addition:

Base-10 example: [3, 15, 7] → [3, 5, 8] (carry the 1 from 15) Base-2^K: [c₀, c₁, c₂] where cⱼ might
overflow 2^K → propagate carries → all cⱼ in [-2^{K-1}, 2^{K-1})

Rescaling (changing precision) is just bit-shifting — you drop or add limbs. Compare this to
RNS-based systems where rescaling requires expensive basis conversion. This is one of poulpy's key
advantages.

---

Lecture 7: Fast Polynomial Multiplication

The Bottleneck

GLWE encryption, decryption, and every homomorphic operation require polynomial multiplication in
Z[X]/(X^N + 1). Naive multiplication is O(N²). With N = 1024 or 2048, this is too slow.

The FFT/NTT Trick

The Discrete Fourier Transform converts a polynomial from coefficient representation to evaluation
representation (values at specific points):

Coefficient domain: p(X) = [a₀, a₁, a₂, ..., a_{N-1}] ↓ Forward FFT/NTT Evaluation domain: [p(ω⁰),
p(ω¹), p(ω²), ..., p(ω^{N-1})] where ω is a primitive root of unity

The key insight: multiplication of polynomials in coefficient domain becomes pointwise
multiplication in evaluation domain:

Coefficient: p(X) × q(X) — O(N²) Evaluation: p(ωⁱ) × q(ωⁱ) — O(N) pointwise, each i independently!

Total: O(N log N) for FFT + O(N) pointwise + O(N log N) for inverse FFT = O(N log N) ≪ O(N²)

Negacyclic Convolution

We need multiplication mod (X^N + 1), not just mod X^N. This requires a negacyclic transform — we
use roots of X^{2N} - 1 that satisfy ω^N = -1, which automatically handles the negation when
wrapping around.

Two Backend Implementations

FFT64 (Floating-Point):

- Uses f64 (64-bit doubles) for the transform
- Fast, simple, but introduces small rounding errors
- Works for moderate precision requirements
- ScalarPrep = f64, ScalarBig = i64

NTT120 (Exact Integer via CRT):

- Uses Number Theoretic Transform — FFT but in modular arithmetic (integers mod a prime)
- Works over 4 primes simultaneously: Q = Q₀ × Q₁ × Q₂ × Q₃ ≈ 2^{120}
- Each prime is ~30 bits, so NTT fits in 32-bit arithmetic
- Chinese Remainder Theorem (CRT) reconstructs the full result
- Exact — no floating-point error whatsoever

NTT120 Pipeline:

Coefficient (i64) ↓ encode CRT residues: [c mod Q₀, c mod Q₁, c mod Q₂, c mod Q₃] (Q120a format) ↓
forward NTT (independently on each prime) NTT domain: [ĉ mod Q₀, ĉ mod Q₁, ĉ mod Q₂, ĉ mod Q₃]
(Q120b format) ↓ pointwise multiply ↓ inverse NTT CRT residues of product ↓ CRT reconstruction
(Barrett reduction, no division) Exact result (i128)

The Three Domains

Every polynomial lives in one of three representations:

┌───────────────────┬───────────┬──────────────┬──────────────────────────────────┐ │ Domain │ Type
│ Scalar │ Purpose │
├───────────────────┼───────────┼──────────────┼──────────────────────────────────┤ │ Coefficient │
VecZnx │ i64 │ Storage, addition, serialization │
├───────────────────┼───────────┼──────────────┼──────────────────────────────────┤ │ DFT/NTT │
VecZnxDft │ f64 or Q120b │ Fast multiplication │
├───────────────────┼───────────┼──────────────┼──────────────────────────────────┤ │ Big
(accumulator) │ VecZnxBig │ i64 or i128 │ IDFT output before normalization │
└───────────────────┴───────────┴──────────────┴──────────────────────────────────┘

Lifecycle: VecZnx ──[forward DFT]──► VecZnxDft ──[pointwise ops]──► VecZnxDft │ VecZnx
◄──[normalize]── VecZnxBig ◄──[inverse DFT]───────────┘

---

Lecture 8: SVP, VMP — The Building Blocks

SVP: Scalar-Vector Product

What: Multiply one polynomial (scalar) against a vector of polynomials.

Where it's used: Encryption and decryption — multiplying by the secret key polynomial.

Setup: scalar s(X) ──[prepare]──► SvpPPol (s in DFT domain, stored once)

Operation: SvpPPol × VecZnxDft ──[pointwise mul per column]──► VecZnxDft

    Conceptually: [v₁, v₂, ..., vₖ] × s = [v₁·s, v₂·s, ..., vₖ·s]
    But done in DFT domain, so each multiplication is O(N) pointwise.

The "prepare" step (one-time DFT of the scalar) amortizes the cost across many multiplications.

VMP: Vector-Matrix Product

What: Multiply a vector of polynomials against a prepared matrix of polynomials.

Where it's used: Keyswitching and external products — the core FHE operations.

Setup: Matrix M ──[prepare]──► VmpPMat (each row in DFT domain)

Operation: VecZnxDft × VmpPMat ──[frequency-domain mat-vec product]──► VecZnxDft

    This is like standard matrix-vector multiplication,
    but each "scalar multiply" is a pointwise DFT multiplication,
    and each "addition" is pointwise DFT addition.

Think of it as: the gadget decomposition digits of the input vector are multiplied against the rows
of the keyswitching/GGSW matrix, all in DFT domain for speed.

---

Lecture 9: Gadget Decomposition — Controlling Noise

The Noise Growth Problem

When you multiply two ciphertexts naively, the noise squares. After a few multiplications, it
overwhelms the message:

After 1 multiplication: noise ≈ e² After 2 multiplications: noise ≈ e⁴ After k multiplications:
noise ≈ e^{2^k}

This grows astronomically fast!

The Gadget Trick

Instead of multiplying ciphertexts directly, we use gadget decomposition to keep noise linear.

Idea: Decompose one operand into small digits, multiply each digit by a pre-encrypted version of the
other operand at the corresponding scale, and sum up.

Analogy — multiplication by decomposition in decimal: Instead of: 1234 × secret

Decompose: 1234 = 1×1000 + 2×100 + 3×10 + 4×1

Compute: 1 × (secret × 1000) ← pre-computed and encrypted + 2 × (secret × 100) ← pre-computed and
encrypted + 3 × (secret × 10) ← pre-computed and encrypted + 4 × (secret × 1) ← pre-computed and
encrypted

The crucial point: the multipliers (1, 2, 3, 4) are small (single digits), so they contribute very
little noise. The pre-encrypted scaled versions of secret are computed once during key generation.

Formal Gadget Decomposition

Parameters:

- dnum — number of digits
- dsize — limbs per digit (each limb is base2k bits)
- Digit width = dsize × base2k bits
- Total precision covered = dnum × dsize × base2k bits

Decomposition of value v with precision k bits:

v = d₀ · β⁰ + d₁ · β¹ + ... + d\_{dnum-1} · β^{dnum-1}

where β = 2^{dsize × base2k} (the digit base) and each dᵢ ∈ [-β/2, β/2) (small!)

Noise analysis: Noise per digit: proportional to dᵢ ≤ β/2 = 2^{dsize·base2k - 1} Total noise:
proportional to dnum × β/2

Trade-off:

- More digits (dnum ↑) → smaller digits (β ↓) → less noise per digit
- But more digits → more additions → more total accumulated noise
- Sweet spot depends on the specific parameters and operation

---

Lecture 10: GGSW and the External Product

GGSW Ciphertext

A GGSW ciphertext encrypts a message m in a special matrix form that enables the external product.
It's a dnum × (rank+1) matrix of GLWE ciphertexts:

GGSW(m) = | GLWE(m · 1 · β⁰) GLWE(m · s₁ · β⁰) ... | ← digit 0 | GLWE(m · 1 · β¹) GLWE(m · s₁ · β¹)
... | ← digit 1 | ... ... ... | | GLWE(m · 1 · β^{d-1}) GLWE(m · s₁ · β^{d-1}) | ← digit d-1

Each row encrypts the message m scaled by the gadget power β^i Each column encrypts m multiplied by
a different component (1, s₁, s₂, ...)

The External Product: GGSW × GLWE

This is the most important operation in TFHE. It multiplies a GLWE ciphertext by a GGSW-encrypted
message:

Input: GLWE(a) and GGSW(m) Output: GLWE(a · m)

Algorithm:

1. Decompose GLWE(a) into digits: a = d₀ + d₁·β + d₂·β² + ... + d\_{dnum-1}·β^{dnum-1}

2. Multiply each digit by the corresponding GGSW row: result = Σᵢ dᵢ × GGSW[i]

3. The gadget powers cancel out: Σᵢ dᵢ × GLWE(m · β^i) = GLWE(m · Σᵢ dᵢ · β^i) = GLWE(m · a)

Why noise stays small: Each digit dᵢ is small (bounded by β/2), so the noise contribution of each
row is proportional to a small number times the GGSW encryption noise. Much better than multiplying
two full-size ciphertexts!

Implementation (how poulpy does it):

1. For each digit dᵢ of the GLWE: dᵢ_dft = FFT(dᵢ) ← coefficient → DFT domain res_dft += VMP(dᵢ_dft,
   GGSW_prepared[i]) ← vector-matrix product in DFT domain
2. res_big = IFFT(res_dft) ← DFT → big accumulator
3. result = normalize(res_big) ← big → coefficient domain

---

Lecture 11: Keyswitching

Why Keyswitching?

Sometimes we need to convert a ciphertext from one secret key to another. This happens when:

- Changing the ring dimension (e.g., LWE ↔ GLWE)
- Changing the rank (e.g., rank 2 → rank 1)
- After bootstrapping (which uses a different key internally)

How It Works

We need a keyswitching key — a GGLWE that encrypts the old secret key s₀ under the new secret key
s₁:

KSK = GGLWE\_{s₁}(s₀)

= for each digit i, for each component j of s₀: GLWE\_{s₁}(s₀[j] · β^i)

Keyswitching operation: Input: ct = (c₀, c₁, ..., c\_{rank_in}) under s₀ Output: ct' under s₁

1. Take the mask components c₁, ..., c\_{rank_in}
2. Decompose them into digits
3. Multiply against the KSK (vector-matrix product): result_dft = VMP(decompose(masks), KSK)
4. Add the body: result = IDFT(result_dft) + c₀
5. Normalize

Intuition: The mask components cᵢ were originally multiplied by sᵢ₀ (old key). The KSK encrypts sᵢ₀
under the new key, so we're effectively re-encrypting the contribution of each mask component.

---

Lecture 12: The CMux Gate

What Is CMux?

CMux (Controlled Multiplexer) is the fundamental gate for homomorphic circuits. It selects between
two values based on an encrypted bit:

CMux(selector, true_val, false_val): if selector = 1: output true_val if selector = 0: output
false_val

Algebraically: result = (true_val - false_val) · selector + false_val = selector · true_val + (1 -
selector) · false_val

Implementation with External Product

- selector is a GGSW ciphertext (encrypting 0 or 1)
- true_val and false_val are GLWE ciphertexts

1. diff = true_val - false_val ← GLWE subtraction (trivial, no noise growth)
2. product = ExternalProduct(selector, diff) ← GGSW × GLWE
3. result = product + false_val ← GLWE addition (trivial)

If selector = 1: result = diff · 1 + false_val = true_val If selector = 0: result = diff · 0 +
false_val = false_val

The beauty: the selector is encrypted. The server doesn't know which value was selected. And the
noise growth is controlled by the gadget decomposition.

---

Lecture 13: Blind Rotation (Programmable Bootstrapping)

The Noise Problem, Revisited

Every homomorphic operation adds noise. After enough operations, the noise overwhelms the message
and decryption fails. We need a way to reset the noise while keeping the data encrypted.

This is bootstrapping — the most expensive but most critical operation in FHE.

What Blind Rotation Does

It evaluates an arbitrary function f on an encrypted input, producing a fresh (low-noise)
ciphertext:

Input: LWE(x) — noisy encryption of x Output: GLWE(f(x)) — fresh encryption of f(x)

The function f is encoded as a Lookup Table (LUT) polynomial.

The word "blind" means the rotation happens without knowing x — everything is done on ciphertexts.

The Algorithm (CGGI Method)

Step 0 — Encode the LUT: Encode the function f as a polynomial where coefficient j stores f(j):

LUT(X) = f(0) + f(1)·X + f(2)·X² + ... + f(2N-1)·X^{2N-1}

To evaluate f(x), we need to rotate this polynomial by x positions so that f(x) appears at
position 0.

Step 1 — Initial rotation by the body: ACC = X^{-b} · LUT (rotate by b, the LWE body, which is
public in the ciphertext)

Step 2 — Accumulate mask contributions: For each LWE mask coefficient aᵢ (there are n_lwe of them):

ACC = ACC + (X^{aᵢ} - 1) · ExternalProduct(BRK[i], ACC)

where BRK[i] is a Blind Rotation Key — a GGSW encryption of the i-th secret key bit sᵢ.

What happens here? Let's trace the logic:

ExternalProduct(BRK[i], ACC) = ACC · sᵢ (homomorphically)

(X^{aᵢ} - 1) · ACC · sᵢ: - If sᵢ = 1: adds (X^{aᵢ} - 1) · ACC to the accumulator → net effect: ACC
gets rotated by aᵢ (because ACC + (X^{aᵢ} - 1)·ACC = X^{aᵢ}·ACC) - If sᵢ = 0: adds 0 (no change)

After all n_lwe iterations: ACC has been rotated by: b + Σ(aᵢ · sᵢ) = b + ⟨a,s⟩ = m + e

Total rotation ≈ m (plus small error from rounding)

Step 3 — Extract result: Coefficient 0 of the final ACC polynomial contains f(m).

Why It's Called "Programmable"

By changing the LUT polynomial, you can compute any function of the encrypted input. Want to compute
AND? Put the AND truth table in the LUT. Want square root? Put square root values in the LUT. This
is incredibly flexible.

Block-Binary Optimization

The standard algorithm does one external product per LWE coefficient (n_lwe products). The
block-binary variant batches multiple coefficients together:

Instead of processing aᵢ one at a time, process block_size coefficients together.

This works when the secret key has a special structure (binary block distribution: exactly one 1 per
block of coefficients).

Result: fewer external products → faster bootstrapping.

---

Lecture 14: Circuit Bootstrapping (GLWE → GGSW)

Why Do We Need This?

After blind rotation, we get a GLWE ciphertext. But for CMux gates, we need GGSW ciphertexts.
Circuit bootstrapping bridges this gap:

Input: GLWE(bit) or LWE(bit) Output: GGSW(bit) — ready for use in CMux gates

The Three-Stage Pipeline

Stage 1: Blind Rotation Run blind rotation with a special LUT that encodes the GGSW row structure:

For each GGSW row i: LUT[j] = j × 2^{base2k × (dnum - 1 - i)}

This produces a GLWE ciphertext whose coefficients encode the scaled message values needed for each
GGSW row.

Stage 2: Trace (Coefficient Extraction via Automorphisms)

The GLWE from Stage 1 has the values we need scattered across different coefficients. We need to
isolate them.

Automorphism σₚ maps X → X^p in the ring. The trace operation applies several automorphisms and
averages the results, which zeros out certain coefficients while preserving others:

Trace*gap(c) = (1/gap) · Σ*{j=0}^{gap-1} σ\_{1+j·(N/gap)}(c)

This keeps every gap-th coefficient and zeros out the rest. Using automorphism keys (pre-computed
GGLWE ciphertexts), this is done homomorphically.

Stage 3: GGLWE-to-GGSW Key-Switch

The trace output is in GGLWE form. A final keyswitching step (using a tensor key) converts it to the
proper GGSW format:

GGLWE intermediate → [tensor key-switch] → GGSW(bit)

End Result

We now have a GGSW ciphertext with fresh noise, ready to be used as a CMux selector. This is the key
enabler for arbitrary circuits.

---

Lecture 15: BDD Arithmetic — Computing on Encrypted Integers

The Goal

We want to perform word-level arithmetic on encrypted integers (u8, u16, u32, u64):

encrypt(a) + encrypt(b) → encrypt(a + b) encrypt(a) × encrypt(b) → encrypt(a × b) encrypt(a) &
encrypt(b) → encrypt(a & b) ...all without decrypting!

Binary Decision Diagrams (BDDs)

A BDD is a compact representation of a Boolean function as a directed acyclic graph. Each internal
node tests one input bit and branches left (0) or right (1):

Example: f(x₁, x₂) = x₁ AND x₂

           x₁
          / \
         0   x₂
            / \
           0   1

Any Boolean function (including addition carry chains, comparisons, etc.) can be represented as a
BDD. Multi-output functions (like 32-bit addition with 32 output bits) are represented as shared
BDDs where different outputs share internal nodes.

The Three-Phase Pipeline

Phase 1: Pack & Encrypt

Each bit of the plaintext integer is encrypted separately, then packed into a single GLWE polynomial
using clever bit interleaving:

FheUint<u32>: 32 bits packed in one GLWE polynomial

Bit layout (interleaved for efficient byte extraction): bit_index(i) = ((i & 7) << LOG_BYTES) |
(i >> 3)

This layout enables extracting whole bytes with a single polynomial rotation + trace.

Phase 2: Prepare (Circuit Bootstrap Each Bit)

For each of the 32 bits:

1. Extract the bit as an LWE ciphertext (GLWE-to-LWE keyswitching)
2. Circuit bootstrap it to a GGSW ciphertext
3. Prepare (DFT-transform) the GGSW

FheUint (packed GLWE) → FheUintPrepared (32 independent GGSWs in DFT domain)

This is embarrassingly parallel — each bit is independent. Poulpy uses multi-threading here, with
dedicated scratch arenas per thread.

Phase 3: Evaluate BDD Circuit

Walk the BDD level by level. At each node:

- CMux node: result = CMux(input_bit[selector], high_child, low_child)
- Copy node: pass through unchanged
- None: unused slot

For each BDD level (top to bottom): For each node in level: if Cmux(bit_idx, hi, lo): state[j] =
CMux(bits[bit_idx], state[hi], state[lo]) if Copy: state[j] = state[j] (unchanged) Swap level
buffers

The final level writes directly into the output GLWE polynomial, producing a fresh FheUint result.

Supported Operations

Each operation has a statically compiled BDD (generated at compile time):

┌───────────────┬───────────────────────────────────────┐ │ Operation │ Description │
├───────────────┼───────────────────────────────────────┤ │ Add │ 32-bit addition with carry chain │
├───────────────┼───────────────────────────────────────┤ │ Sub │ 32-bit subtraction │
├───────────────┼───────────────────────────────────────┤ │ Sll, Srl, Sra │ Shift left/right
(logical/arithmetic) │ ├───────────────┼───────────────────────────────────────┤ │ And, Or, Xor │
Bitwise operations │ ├───────────────┼───────────────────────────────────────┤ │ Slt, Sltu │
Signed/unsigned less-than comparison │ └───────────────┴───────────────────────────────────────┘

Blind Selection

An additional primitive: select from 2^k encrypted values based on an encrypted index:

glwe_blind_selection(index_bits, values[0..2^k]): Uses a tree of CMux gates (log₂ depth) Output:
values[index] — without revealing which index was selected

This enables oblivious array access — critical for privacy-preserving computation.

---

Lecture 16: Putting It All Together — End-to-End Flow

The Complete Pipeline

                      ┌─────────────────────────────────────┐
                      │           KEY GENERATION             │
                      │                                     │
                      │  Secret Key (s)                     │
                      │    ↓                                │
                      │  Blind Rotation Keys (BRK)          │
                      │  = GGSW encryptions of s bits       │
                      │    ↓                                │
                      │  Automorphism Keys (ATK)            │
                      │  = GGLWE for trace operations       │
                      │    ↓                                │
                      │  Tensor Switch Key (TSK)            │
                      │  = for GGLWE→GGSW conversion       │
                      │    ↓                                │
                      │  Keyswitching Keys (KSK)            │
                      │  = for GLWE↔LWE conversion          │
                      └─────────────────────────────────────┘
                                      │
                      ┌───────────────┴───────────────┐
                      ▼                               ▼
              ┌──────────────┐               ┌──────────────┐
              │   CLIENT     │               │   SERVER     │
              │              │               │              │
              │  plaintext   │               │              │
              │  = 42 (u32)  │               │              │
              │    ↓         │   send ct     │              │
              │  encrypt     │ ────────────► │  FheUint     │
              │  each bit    │   send keys   │  (packed)    │
              │              │ ────────────► │    ↓         │
              │              │               │  prepare     │
              │              │               │  (circuit    │
              │              │               │   bootstrap  │
              │              │               │   each bit)  │
              │              │               │    ↓         │
              │              │               │  FheUint-    │
              │              │               │  Prepared    │
              │              │               │  (32 GGSWs)  │
              │              │               │    ↓         │
              │              │               │  evaluate    │
              │              │               │  BDD circuit │
              │              │               │  (CMux gates)│
              │              │               │    ↓         │
              │  decrypt  ◄──────────────────│  FheUint     │
              │  = 84     │  send result     │  (result)    │
              │           │                  │              │
              └───────────┘                  └──────────────┘

Noise Budget Through the Pipeline

Operation Noise Level ────────────────────────────────────────────── Fresh encryption █░░░░░░░░░
(tiny: σ = 3.2)  
 After GLWE addition █░░░░░░░░░ (adds linearly)  
 After external product ██░░░░░░░░ (controlled by gadget)  
 After blind rotation ████░░░░░░ (n_lwe external products)  
 After circuit bootstrapping ██░░░░░░░░ (REFRESHED! lower noise)  
 After BDD circuit evaluation ████░░░░░░ (CMux chain)  
 After another bootstrapping ██░░░░░░░░ (REFRESHED again)  
 ──────────  
 Decryption threshold: past here → errors

The key insight: bootstrapping resets the noise. As long as the noise before bootstrapping stays
below the threshold, computation can continue indefinitely.

---

Lecture 17: Memory & Performance Architecture

Three-Layer Layout System

Every ciphertext exists in three forms, optimized for different use cases:

┌─────────────┐ ┌─────────────┐ ┌─────────────┐  
 │ STANDARD │ │ COMPRESSED │ │ PREPARED │ │ │ │ │ │ │  
 │ VecZnx │ │ VecZnx + │ │ VecZnxDft │ │ (i64 limbs) │ │ 32-byte │ │ (DFT domain)│  
 │ │ │ seed │ │ │  
 │ Full size │ │ ~½ size │ │ Backend- │  
 │ Portable │ │ Portable │ │ specific │  
 │ Serializable│ │ Serializable│ │ Fast ops │ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘  
 │ │ │  
 │ For computation │ For storage │ For multiplication  
 │ and I/O │ and transfer │ and keyswitching

Compressed form: the random mask polynomials are replaced by a 32-byte seed. The mask can be
regenerated from the seed using a PRNG. This nearly halves storage  
 cost.

Prepared form: polynomials are transformed to DFT/NTT domain. This is the form used during
computation — all multiplications become pointwise and fast.

Scratch Allocation (Zero Heap Allocation)

Poulpy never allocates heap memory during cryptographic operations. Instead, it uses arena-style
scratch allocation:

1. Before computation: allocate one big scratch buffer  
   let scratch = module.scratch_alloc(required_bytes);

2. During computation: carve temporary slices from the buffer  
   let (tmp, remaining_scratch) = scratch.take_slice(size);  
   // use tmp...  
   // tmp is automatically "freed" when this operation returns

3. Reuse the same buffer for the next operation

Why? Memory allocation is slow and unpredictable. In cryptographic workloads with many repeated
operations (thousands of external products during bootstrapping), avoiding allocation overhead is
critical for performance.

Backend Abstraction

The entire architecture is backend-agnostic:

┌──────────────────────────────────────────────┐  
 │ poulpy-schemes │ │ (blind rotation, CMux, BDD) │  
 ├──────────────────────────────────────────────┤ │ poulpy-core │  
 │ (GLWE, GGSW, external product, keyswitch) │ ├──────────────────────────────────────────────┤  
 │ poulpy-hal │ │ (API traits: VecZnxAdd, VmpApply, ...) │  
 ├───────────────────┬──────────────────────────┤  
 │ poulpy-cpu-ref │ poulpy-cpu-avx │ │ (portable f64 │ (AVX2 + FMA │  
 │ or NTT120) │ accelerated) │  
 └───────────────────┴──────────────────────────┘

Write your FHE scheme code once. Swap the backend by changing a type parameter. Future GPU or FPGA
backends can be added without touching the scheme code.

---

Lecture 18: Exam Review — Key Concepts Cheat Sheet

┌───────────────────────┬──────────────────────────────────────────────────────────────────────┐ │
Concept │ One-Line Summary │  
 ├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Torus │ Real numbers mod 1; wrapping arithmetic for FHE │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Base-2^K limbs │ Decompose Torus elements into small digits for precision control │  
 ├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ LWE │ b = ⟨a,s⟩ + m + e; hardness from noise │  
 ├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ GLWE │ Polynomial-ring LWE; N message slots per ciphertext │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ GGSW │ Matrix of GLWE encrypting scaled message; enables external product │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Gadget decomposition │ Break values into small digits to control noise growth │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ External product │ GGSW × GLWE → GLWE; core FHE multiply with controlled noise │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Keyswitching │ Convert ciphertext from one key to another via GGLWE │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ CMux │ sel ? t : f on encrypted data; uses external product │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ NTT/FFT │ Fast polynomial multiplication in O(N log N) │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Blind rotation │ Evaluate LUT on encrypted input; refreshes noise │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Circuit bootstrapping │ GLWE → GGSW conversion via blind rotation + trace │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ BDD arithmetic │ Word-level ops via decision diagrams of CMux gates │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Normalization │ Carry propagation between base-2^K limbs │
├───────────────────────┼──────────────────────────────────────────────────────────────────────┤  
 │ Three layouts │ Standard (storage) → Compressed (transport) → Prepared (computation) │
└───────────────────────┴──────────────────────────────────────────────────────────────────────┘

Common Exam Questions

Q: Why does FHE need noise?  
 A: Without noise, LWE reduces to a system of linear equations, solvable in polynomial time. Noise
makes the problem equivalent to hard lattice problems.

Q: Why does noise grow during computation? A: Addition increases noise linearly. Multiplication
(external product) multiplies the noise by the decomposed digits. The gadget decomposition keeps
these  
 digits small, but noise still accumulates.

Q: How does bootstrapping solve the noise problem?  
 A: Bootstrapping evaluates the decryption function homomorphically, producing a fresh ciphertext
with low noise. The output encrypts the same message but with reset noise, enabling unlimited
further computation.

Q: What is the advantage of the bivariate polynomial representation over RNS?  
 A: (1) Rescaling is a simple bit-shift, not an expensive basis conversion. (2) Digit decomposition
is O(n), not O(n²). (3) Deterministic computation — no floating-point non-determinism. (4) Unified
plaintext space across all operations.

Q: Why separate FFT64 and NTT120 backends?  
 A: FFT64 (floating-point) is faster but approximate — fine for moderate precision. NTT120 (integer
CRT) is exact — needed when precision requirements are strict or when deterministic results are
mandatory.

Q: What makes the external product efficient?  
 A: Gadget decomposition limits each digit's size, the GGSW pre-stores scaled encryptions at each
gadget level, VMP in DFT domain makes the actual multiplication O(N log N), and scratch allocation
avoids heap overhead.

---

Good luck on the exam! The key to understanding FHE is following the noise: every design decision —
from gadget decomposition to bootstrapping to the bivariate representation — exists to keep noise
under control while enabling computation on encrypted data.

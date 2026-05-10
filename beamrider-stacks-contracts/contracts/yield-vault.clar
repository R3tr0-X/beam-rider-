;; yield-vault
;;
;; BeamRider's Stacks-side custody and venue dispatch.
;;
;; Holds STX and SIP-010 sBTC for the agent operator. Routes to one of three
;; allow-listed venues — Zest, Bitflow LP, StackingDAO — via dedicated
;; entry points. There is **no** generic "call any contract" function: that
;; surface is an arbitrary-call hole and the operator does not need it.
;;
;; Each entry point emits a typed `print` event so the off-chain Rust
;; agent can index deposits without needing a private RPC.

(define-trait ft-trait
    (
        (transfer (uint principal principal (optional (buff 34))) (response bool uint))
        (get-balance (principal) (response uint uint))
    )
)

;; Venues are addressed as opaque target principals + an action tag the
;; off-chain indexer uses to label the deposit. The actual venue contract's
;; deposit selector is invoked in a *typed* sub-function per venue, so we
;; never hand out a raw `contract-call?` to caller-supplied principals.

(define-constant ERR-NOT-OWNER          (err u401))
(define-constant ERR-INVALID-AMOUNT     (err u402))
(define-constant ERR-INVALID-TARGET     (err u403))
(define-constant ERR-VENUE-NOT-ALLOWED  (err u404))
(define-constant ERR-STX-TRANSFER       (err u500))
(define-constant ERR-FT-TRANSFER        (err u501))

(define-constant VENUE-ZEST          u1)
(define-constant VENUE-BITFLOW-LP    u2)
(define-constant VENUE-STACKING-DAO  u3)

(define-data-var contract-owner principal tx-sender)

;; Per-venue allow-listed deposit target (the venue contract's principal).
;; Off-chain operator pins these at deploy + can update if the venue migrates.
(define-map venue-targets { venue: uint } { target: principal })

(define-private (is-owner)
    (is-eq tx-sender (var-get contract-owner))
)

(define-private (require-target (venue uint))
    (match (map-get? venue-targets { venue: venue })
        entry (ok (get target entry))
        ERR-VENUE-NOT-ALLOWED
    )
)

(define-public (set-venue-target (venue uint) (target principal))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (or (is-eq venue VENUE-ZEST)
                      (is-eq venue VENUE-BITFLOW-LP)
                      (is-eq venue VENUE-STACKING-DAO))
            ERR-VENUE-NOT-ALLOWED)
        (map-set venue-targets { venue: venue } { target: target })
        (print { event: "venue-target-set", venue: venue, target: target })
        (ok true)
    )
)

(define-public (transfer-ownership (new-owner principal))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (var-set contract-owner new-owner)
        (print { event: "ownership-transferred", new-owner: new-owner })
        (ok true)
    )
)

;; Deposit STX to a venue target (e.g. StackingDAO delegate).
;;
;; The vault holds STX directly; we transfer to the venue target and let the
;; off-chain indexer correlate the print-event with the venue's own logs.
(define-public (deposit-stx (venue uint) (amount uint))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (let ((target (try! (require-target venue))))
            (match (as-contract (stx-transfer? amount tx-sender target))
                ok-result
                    (begin
                        (print {
                            event:  "venue-deposit",
                            venue:  venue,
                            token:  "stx",
                            target: target,
                            amount: amount
                        })
                        (ok true)
                    )
                err-code (err err-code)
            )
        )
    )
)

;; Deposit a SIP-010 token (sBTC) to a venue target (e.g. Zest pool).
(define-public (deposit-ft (venue uint) (token <ft-trait>) (amount uint))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (let ((target (try! (require-target venue))))
            (match (as-contract (contract-call? token transfer amount tx-sender target none))
                ok-result
                    (begin
                        (print {
                            event:  "venue-deposit",
                            venue:  venue,
                            token:  (contract-of token),
                            target: target,
                            amount: amount
                        })
                        (ok true)
                    )
                err-code (err err-code)
            )
        )
    )
)

;; Owner withdraw of STX held by the vault (e.g. emergency, harvest).
(define-public (withdraw-stx (recipient principal) (amount uint))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (match (as-contract (stx-transfer? amount tx-sender recipient))
            ok-result
                (begin
                    (print { event: "withdraw", token: "stx", recipient: recipient, amount: amount })
                    (ok true)
                )
            err-code (err err-code)
        )
    )
)

;; Owner withdraw of a SIP-010 token held by the vault.
(define-public (withdraw-ft (token <ft-trait>) (recipient principal) (amount uint))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (match (as-contract (contract-call? token transfer amount tx-sender recipient none))
            ok-result
                (begin
                    (print {
                        event:     "withdraw",
                        token:     (contract-of token),
                        recipient: recipient,
                        amount:    amount
                    })
                    (ok true)
                )
            err-code (err err-code)
        )
    )
)

(define-read-only (get-venue-target (venue uint))
    (map-get? venue-targets { venue: venue })
)

(define-read-only (get-owner)
    (var-get contract-owner)
)

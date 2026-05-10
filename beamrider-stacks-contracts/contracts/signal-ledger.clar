;; signal-ledger
;;
;; Stacks-side payment receipts for BeamRider signals.
;;
;; Buyers transfer STX or a SIP-010 token (sBTC) to the configured receiver
;; and a print-event is emitted. Replay protection is keyed on `sale-id`.
;;
;; Off-chain, the BeamRider Rust agent watches `print` events on this
;; contract via the Hiro API and gates the signed signal payload on a
;; matching event for the buyer's principal.

(define-trait ft-trait
    (
        (transfer (uint principal principal (optional (buff 34))) (response bool uint))
        (get-balance (principal) (response uint uint))
    )
)

(define-constant ERR-NOT-OWNER          (err u401))
(define-constant ERR-DUPLICATE-SALE     (err u409))
(define-constant ERR-INVALID-SALE-ID    (err u400))
(define-constant ERR-INVALID-PAIR       (err u402))
(define-constant ERR-INVALID-AMOUNT     (err u403))
(define-constant ERR-INVALID-RECEIVER   (err u404))
(define-constant ERR-TOKEN-NOT-ALLOWED  (err u405))
(define-constant ERR-STX-TRANSFER       (err u500))
(define-constant ERR-FT-TRANSFER        (err u501))

(define-data-var contract-owner principal tx-sender)
(define-data-var agent-receiver principal tx-sender)

;; Allow-list of SIP-010 token principals accepted as payment.
(define-map allowed-tokens { token: principal } { min-amount: uint })

;; sale-id → recorded marker. Replay-protection.
(define-map sales { sale-id: (buff 32) } { block-height: uint })

(define-private (is-owner)
    (is-eq tx-sender (var-get contract-owner))
)

(define-private (is-valid-pair (pair (string-utf8 20)))
    (let ((n (len pair)))
        (and (> n u0) (<= n u20))
    )
)

(define-private (is-valid-sale-id (sale-id (buff 32)))
    (and
        (is-eq (len sale-id) u32)
        (not (is-eq sale-id 0x0000000000000000000000000000000000000000000000000000000000000000))
    )
)

(define-public (set-receiver (new-receiver principal))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (var-set agent-receiver new-receiver)
        (print { event: "receiver-updated", receiver: new-receiver })
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

(define-public (allow-token (token principal) (min-amount uint))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (asserts! (> min-amount u0) ERR-INVALID-AMOUNT)
        (map-set allowed-tokens { token: token } { min-amount: min-amount })
        (print { event: "token-allowed", token: token, min-amount: min-amount })
        (ok true)
    )
)

(define-public (disallow-token (token principal))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (map-delete allowed-tokens { token: token })
        (print { event: "token-disallowed", token: token })
        (ok true)
    )
)

(define-public (buy-signal-stx
        (sale-id  (buff 32))
        (agent-id uint)
        (pair     (string-utf8 20))
        (amount   uint))
    (begin
        (asserts! (is-valid-sale-id sale-id) ERR-INVALID-SALE-ID)
        (asserts! (is-valid-pair pair) ERR-INVALID-PAIR)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (asserts! (is-none (map-get? sales { sale-id: sale-id })) ERR-DUPLICATE-SALE)

        ;; Effects before interactions.
        (map-set sales { sale-id: sale-id } { block-height: block-height })

        (let ((receiver (var-get agent-receiver)))
            (asserts! (not (is-eq receiver (as-contract tx-sender))) ERR-INVALID-RECEIVER)
            (match (stx-transfer? amount tx-sender receiver)
                ok-result
                    (begin
                        (print {
                            event:        "signal-sale",
                            sale-id:      sale-id,
                            agent-id:     agent-id,
                            pair:         pair,
                            buyer:        tx-sender,
                            token:        "stx",
                            amount:       amount,
                            block-height: block-height
                        })
                        (ok true)
                    )
                err-code (err err-code)
            )
        )
    )
)

(define-public (buy-signal-ft
        (token    <ft-trait>)
        (sale-id  (buff 32))
        (agent-id uint)
        (pair     (string-utf8 20))
        (amount   uint))
    (let ((token-principal (contract-of token)))
        (asserts! (is-valid-sale-id sale-id) ERR-INVALID-SALE-ID)
        (asserts! (is-valid-pair pair) ERR-INVALID-PAIR)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (asserts! (is-none (map-get? sales { sale-id: sale-id })) ERR-DUPLICATE-SALE)

        (let ((entry (unwrap! (map-get? allowed-tokens { token: token-principal }) ERR-TOKEN-NOT-ALLOWED)))
            (asserts! (>= amount (get min-amount entry)) ERR-INVALID-AMOUNT)

            (map-set sales { sale-id: sale-id } { block-height: block-height })

            (let ((receiver (var-get agent-receiver)))
                (match (contract-call? token transfer amount tx-sender receiver none)
                    ok-result
                        (begin
                            (print {
                                event:        "signal-sale",
                                sale-id:      sale-id,
                                agent-id:     agent-id,
                                pair:         pair,
                                buyer:        tx-sender,
                                token:        token-principal,
                                amount:       amount,
                                block-height: block-height
                            })
                            (ok true)
                        )
                    err-code (err err-code)
                )
            )
        )
    )
)

(define-read-only (get-receiver)
    (var-get agent-receiver)
)

(define-read-only (get-owner)
    (var-get contract-owner)
)

(define-read-only (is-allowed-token (token principal))
    (is-some (map-get? allowed-tokens { token: token }))
)

(define-read-only (get-token-min-amount (token principal))
    (match (map-get? allowed-tokens { token: token })
        entry (some (get min-amount entry))
        none
    )
)

(define-read-only (is-recorded (sale-id (buff 32)))
    (is-some (map-get? sales { sale-id: sale-id }))
)

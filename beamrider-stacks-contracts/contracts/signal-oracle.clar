;; signal-oracle
;;
;; Hermes-pattern signal commitment store.
;;
;; The BeamRider Rust agent's off-chain relay posts (pair, hash, confidence)
;; tuples to this contract. Stacks-side buyers can independently read the
;; commitment for the most recent block height and compare it against the
;; signal payload returned by the API. This anchors EVM-side signal
;; authenticity to Bitcoin finality (via Stacks' anchor-block model).

(define-constant ERR-NOT-RELAYER       (err u401))
(define-constant ERR-NOT-OWNER         (err u403))
(define-constant ERR-INVALID-PAIR      (err u402))
(define-constant ERR-INVALID-HASH      (err u404))
(define-constant ERR-INVALID-CONF      (err u405))
(define-constant ERR-NO-COMMITMENT     (err u410))

;; bps domain check upper bound (10_000 bps = 100%).
(define-constant MAX-CONFIDENCE-BPS u10000)

(define-data-var contract-owner       principal tx-sender)
(define-data-var authorized-relayer   principal tx-sender)

;; (pair, block-height) → commitment.
(define-map signal-commitments
    { pair: (string-utf8 20), block-height: uint }
    {
        hash:           (buff 32),
        confidence-bps: uint,
        relayer:        principal
    }
)

;; latest commitment per pair, for cheap last-known reads.
(define-map latest-by-pair
    { pair: (string-utf8 20) }
    {
        block-height:   uint,
        hash:           (buff 32),
        confidence-bps: uint
    }
)

(define-private (is-owner)
    (is-eq tx-sender (var-get contract-owner))
)

(define-private (is-relayer)
    (is-eq tx-sender (var-get authorized-relayer))
)

(define-private (is-valid-pair (pair (string-utf8 20)))
    (let ((n (len pair)))
        (and (> n u0) (<= n u20))
    )
)

(define-private (is-valid-hash (h (buff 32)))
    (and
        (is-eq (len h) u32)
        (not (is-eq h 0x0000000000000000000000000000000000000000000000000000000000000000))
    )
)

(define-public (set-relayer (new-relayer principal))
    (begin
        (asserts! (is-owner) ERR-NOT-OWNER)
        (var-set authorized-relayer new-relayer)
        (print { event: "relayer-updated", relayer: new-relayer })
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

(define-public (commit-signal
        (pair           (string-utf8 20))
        (hash           (buff 32))
        (confidence-bps uint))
    (begin
        (asserts! (is-relayer) ERR-NOT-RELAYER)
        (asserts! (is-valid-pair pair) ERR-INVALID-PAIR)
        (asserts! (is-valid-hash hash) ERR-INVALID-HASH)
        (asserts! (<= confidence-bps MAX-CONFIDENCE-BPS) ERR-INVALID-CONF)

        (map-set signal-commitments
            { pair: pair, block-height: block-height }
            {
                hash:           hash,
                confidence-bps: confidence-bps,
                relayer:        tx-sender
            }
        )
        (map-set latest-by-pair
            { pair: pair }
            {
                block-height:   block-height,
                hash:           hash,
                confidence-bps: confidence-bps
            }
        )
        (print {
            event:          "signal-committed",
            pair:           pair,
            hash:           hash,
            confidence-bps: confidence-bps,
            block-height:   block-height
        })
        (ok true)
    )
)

(define-read-only (get-commitment (pair (string-utf8 20)) (height uint))
    (map-get? signal-commitments { pair: pair, block-height: height })
)

(define-read-only (get-latest (pair (string-utf8 20)))
    (map-get? latest-by-pair { pair: pair })
)

(define-read-only (verify-commitment
        (pair          (string-utf8 20))
        (height        uint)
        (expected-hash (buff 32)))
    (match (map-get? signal-commitments { pair: pair, block-height: height })
        entry (ok (is-eq (get hash entry) expected-hash))
        ERR-NO-COMMITMENT
    )
)

(define-read-only (get-relayer)
    (var-get authorized-relayer)
)

(define-read-only (get-owner)
    (var-get contract-owner)
)

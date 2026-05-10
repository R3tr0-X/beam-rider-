;; beamrider-registry
;;
;; BeamRider agent identity registry on Stacks.
;;
;; Mirrors `BeamRiderRegistry.sol` on Celo. The `agent-id` is *supplied by the
;; caller* and MUST equal the agent's Celo `tokenId`. First-write wins; no
;; reuse, no off-by-one drift between chains.
;;
;; Surface kept deliberately minimal — `register-agent`, `update-metadata`,
;; `transfer-ownership`. No SIP-009 NFT shape: the agent identity is a record,
;; not a transferable token.

(define-constant ERR-INVALID-AGENT-ID    (err u400))
(define-constant ERR-INVALID-PUBKEY      (err u401))
(define-constant ERR-INVALID-NAME        (err u402))
(define-constant ERR-INVALID-URL         (err u403))
(define-constant ERR-AGENT-EXISTS        (err u409))
(define-constant ERR-UNKNOWN-AGENT       (err u404))
(define-constant ERR-NOT-OWNER           (err u405))
(define-constant ERR-INVALID-NEW-OWNER   (err u406))

(define-data-var total-agents uint u0)

(define-map agents
    { agent-id: uint }
    {
        owner:       principal,
        pubkey:      (buff 32),
        name:        (string-utf8 64),
        service-url: (string-utf8 256)
    }
)

(define-private (is-valid-pubkey (pk (buff 32)))
    (not (is-eq pk 0x0000000000000000000000000000000000000000000000000000000000000000))
)

(define-private (is-valid-name (name (string-utf8 64)))
    (let ((n (len name)))
        (and (> n u0) (<= n u64))
    )
)

(define-private (is-valid-url (url (string-utf8 256)))
    (let ((n (len url)))
        (and (> n u0) (<= n u256))
    )
)

(define-public (register-agent
        (agent-id    uint)
        (pubkey      (buff 32))
        (name        (string-utf8 64))
        (service-url (string-utf8 256)))
    (begin
        (asserts! (> agent-id u0) ERR-INVALID-AGENT-ID)
        (asserts! (is-valid-pubkey pubkey) ERR-INVALID-PUBKEY)
        (asserts! (is-valid-name name) ERR-INVALID-NAME)
        (asserts! (is-valid-url service-url) ERR-INVALID-URL)
        (asserts! (is-none (map-get? agents { agent-id: agent-id })) ERR-AGENT-EXISTS)

        (map-set agents
            { agent-id: agent-id }
            {
                owner:       tx-sender,
                pubkey:      pubkey,
                name:        name,
                service-url: service-url
            }
        )
        (var-set total-agents (+ (var-get total-agents) u1))
        (print {
            event:       "agent-registered",
            agent-id:    agent-id,
            owner:       tx-sender,
            pubkey:      pubkey,
            name:        name,
            service-url: service-url
        })
        (ok agent-id)
    )
)

(define-public (update-metadata
        (agent-id    uint)
        (pubkey      (buff 32))
        (name        (string-utf8 64))
        (service-url (string-utf8 256)))
    (let ((existing (unwrap! (map-get? agents { agent-id: agent-id }) ERR-UNKNOWN-AGENT)))
        (asserts! (is-eq tx-sender (get owner existing)) ERR-NOT-OWNER)
        (asserts! (is-valid-pubkey pubkey) ERR-INVALID-PUBKEY)
        (asserts! (is-valid-name name) ERR-INVALID-NAME)
        (asserts! (is-valid-url service-url) ERR-INVALID-URL)

        (map-set agents
            { agent-id: agent-id }
            {
                owner:       (get owner existing),
                pubkey:      pubkey,
                name:        name,
                service-url: service-url
            }
        )
        (print {
            event:       "agent-metadata-updated",
            agent-id:    agent-id,
            pubkey:      pubkey,
            name:        name,
            service-url: service-url
        })
        (ok true)
    )
)

(define-public (transfer-ownership (agent-id uint) (new-owner principal))
    (let ((existing (unwrap! (map-get? agents { agent-id: agent-id }) ERR-UNKNOWN-AGENT)))
        (asserts! (is-eq tx-sender (get owner existing)) ERR-NOT-OWNER)
        (asserts! (not (is-eq new-owner (get owner existing))) ERR-INVALID-NEW-OWNER)

        (map-set agents
            { agent-id: agent-id }
            (merge existing { owner: new-owner })
        )
        (print {
            event:          "agent-ownership-transferred",
            agent-id:       agent-id,
            previous-owner: (get owner existing),
            new-owner:      new-owner
        })
        (ok true)
    )
)

(define-read-only (get-agent (agent-id uint))
    (map-get? agents { agent-id: agent-id })
)

(define-read-only (get-pubkey (agent-id uint))
    (match (map-get? agents { agent-id: agent-id })
        agent (ok (get pubkey agent))
        ERR-UNKNOWN-AGENT
    )
)

(define-read-only (get-owner (agent-id uint))
    (match (map-get? agents { agent-id: agent-id })
        agent (ok (get owner agent))
        ERR-UNKNOWN-AGENT
    )
)

(define-read-only (get-total-agents)
    (var-get total-agents)
)

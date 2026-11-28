-- Dossier d’où une lettre est partie vers la Corbeille ou les Indésirables :
-- « Restaurer » l’y ramène tant que ce dossier existe.
ALTER TABLE messages ADD COLUMN origin_folder INTEGER REFERENCES folders(id) ON DELETE SET NULL;
-- Une lettre que l’utilisateur a replacée dans la boîte de réception s’y lit
-- comme le courrier antérieur au rattachement, sans repasser par le tri.
ALTER TABLE messages ADD COLUMN placed INTEGER NOT NULL DEFAULT 0 CHECK (placed IN (0,1));

DROP VIEW routed_messages;
CREATE VIEW routed_messages AS
SELECT m.*, d.state AS screening_state,
       CASE WHEN f.role='inbox' AND m.uidvalidity=b.uidvalidity THEN
           CASE WHEN d.state='accepte' THEN d.destination
                WHEN d.state IS NULL AND (m.uid<b.uidnext OR m.placed) THEN 'reception' END
       END AS destination
FROM messages m
JOIN folders f ON f.account_id=m.account_id AND f.id=m.folder_id AND f.uidvalidity=m.uidvalidity
JOIN inbox_markers b ON b.account_id=m.account_id
LEFT JOIN sender_decisions d ON d.account_id=m.account_id AND d.address=m.sender_address
    AND f.role='inbox' AND m.uidvalidity=b.uidvalidity
    AND ((m.uid>=b.uidnext AND NOT m.placed) OR d.state!='en_attente')
WHERE NOT m.moving;

-- Une suppression définitive efface ses lettres au lieu de les déplacer.
ALTER TABLE operation_kinds ADD COLUMN expunges INTEGER NOT NULL DEFAULT 0 CHECK (expunges IN (0,1));
INSERT INTO operation_kinds(kind, expunges) VALUES
    ('corbeille',0),('restaurer',0),('supprimer_definitivement',1),('vider_dossier',1);

-- Dossier d’arrivée propre à une lettre. À défaut, celui de l’opération s’applique.
ALTER TABLE operation_messages ADD COLUMN destination INTEGER REFERENCES folders(id) ON DELETE SET NULL;

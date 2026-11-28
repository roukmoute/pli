-- Une copie laisse ses lettres en place et les ajoute au dossier d’arrivée.
ALTER TABLE operation_kinds ADD COLUMN copies INTEGER NOT NULL DEFAULT 0 CHECK (copies IN (0,1));
INSERT INTO operation_kinds(kind, copies) VALUES ('deplacer',0),('copier',1);

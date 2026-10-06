-- Seating reference seeded verbatim from DOC-20260706-WA0002..xlsx [X].
-- Total = 38,181 seats. Buffer (zone tampon) and Presse are NOT sold but are
-- still scanned for access, so kept with sellable=false.

INSERT INTO zone (code, name, kind, sellable, display_order) VALUES
  ('Z1',   'Zone 1 (Sud)',        'public',   true,  1),
  ('Z2',   'Zone 2 (Centre)',     'public',   true,  2),
  ('Z3',   'Zone 3 (Nord)',       'public',   true,  3),
  ('VIP',  'VIP',                 'vip',      true,  4),
  ('VVIP', 'VVIP',                'vvip',     true,  5),
  ('OFF',  'Tribune Officielle',  'official', true,  6),
  ('BUF',  'Zone tampon',         'buffer',   false, 7),
  ('PRESS','Presse',              'press',    false, 8);

-- Sections / levels with exact capacities and gate ranges [X]
INSERT INTO tribune_section (zone_id, label, level, gate_range, capacity, sellable)
SELECT z.id, v.label, v.level, v.gate_range, v.capacity, v.sellable
FROM (VALUES
  ('Z1',   'Zone 1 Supérieur',     'superieur', 'V19-V25',           5114, true),
  ('Z1',   'Zone 1 Inférieur',     'inferieur', NULL,                4845, true),
  ('Z2',   'Zone 2 Supérieur',     'superieur', 'V11-V18',           6314, true),
  ('Z2',   'Zone 2 Inférieur',     'inferieur', NULL,                4150, true),
  ('Z3',   'Zone 3 Supérieur',     'superieur', 'V04-V10',           5114, true),
  ('Z3',   'Zone 3 Inférieur',     'inferieur', NULL,                4572, true),
  ('VIP',  'Mezghena A',           NULL,        'VIP',                699, true),
  ('VIP',  'Mezghena B',           NULL,        'VIP',                699, true),
  ('VVIP', 'Doyen A',              NULL,        'VVIP',               364, true),
  ('VVIP', 'Doyen B',              NULL,        'VVIP',               364, true),
  ('VVIP', 'Historia',             NULL,        'VVIP',               360, true),
  ('OFF',  'Tribune Officielle',   NULL,        NULL,                 148, true),
  ('BUF',  'Zone tampon',          NULL,        'V26-V29/V00-V03',   5438, false),
  ('PRESS','Presse',               NULL,        'V29',                218, false)
) AS v(zone_code, label, level, gate_range, capacity, sellable)
JOIN zone z ON z.code = v.zone_code;

-- Sanity check, faithful to [X]. NOTE a discrepancy inside the source sheet:
-- the printed "Total" is 38,181, but that equals every section EXCEPT Presse
-- (218). The full arithmetic sum of all listed sections is 38,399. Both are
-- asserted so the seed cannot drift from the file. The 218 gap (Presse, "non
-- mise en vente", + 28 box) is flagged for SOGISL — blueprint open question #6.
DO $$
DECLARE all_sections int; sheet_total int;
BEGIN
  SELECT sum(capacity) INTO all_sections FROM tribune_section;
  SELECT sum(capacity) INTO sheet_total FROM tribune_section
    WHERE zone_id <> (SELECT id FROM zone WHERE code = 'PRESS');
  IF all_sections <> 38399 THEN
    RAISE EXCEPTION 'section sum % != 38399 expected from [X]', all_sections;
  END IF;
  IF sheet_total <> 38181 THEN
    RAISE EXCEPTION 'sheet total % != 38181 (fiche technique printed total)', sheet_total;
  END IF;
END $$;

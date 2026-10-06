-- RBAC roles [M §9.1]. Agents are linked to these via agent_role.
INSERT INTO role (code, name) VALUES
  ('admin',      'Administrateur'),
  ('operator',   'Opérateur'),
  ('gate_agent', 'Agent de contrôle')
ON CONFLICT (code) DO NOTHING;

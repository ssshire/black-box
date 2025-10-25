CREATE TABLE users (
  user_id SERIAL PRIMARY KEY,
  username VARCHAR(50) NOT NULL UNIQUE,
  email VARCHAR(100) NOT NULL UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  role VARCHAR(20) NOT NULL DEFAULT 'User'
);

CREATE TABLE sessions (
  session_id SERIAL PRIMARY KEY,
  session_name VARCHAR(100) NOT NULL,
  created_by INT REFERENCES users(user_id),
  created_at TIMESTAMP DEFAULT NOW(),
  max_size INT DEFAULT 50
);

CREATE TABLE messages (
  message_id SERIAL PRIMARY KEY,
  session_id INT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
  user_id INT NOT NULL REFERENCES users(user_id),
  content TEXT NOT NULL,
  sent_at TIMESTAMP DEFAULT NOW()
);


-- =====================
-- 1. INSERT USERS
-- =====================

-- Moderators
INSERT INTO users (username, email, password_hash, role) 
VALUES 
  ('admin_sarah', 'sarah@msgplatform.com', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYIq7bCzb8i', 'Moderator'),
  ('mod_james', 'james@msgplatform.com', '$2b$12$KQv4d2zrCXWIylf1MIBlDPZy7UuyNRKsrO9/MfxZ6HzZJr8dDzc9j', 'Moderator');

-- Regular Users
INSERT INTO users (username, email, password_hash, role) 
VALUES 
  ('alice_wonder', 'alice@email.com', '$2b$12$MRv5e3AsEYXJzmg2NJCmEQAz8VvzORLtsPa/NgYA7IaAKs9eEAd0k', 'User'),
  ('bob_builder', 'bob@email.com', '$2b$12$NSwfd4BtFZYKAnH3OKDnFRBa9WwaOSMutQb/OhZB8JbBLt0fFBe1l', 'User'),
  ('charlie_brown', 'charlie@email.com', '$2b$12$OTxge5CuGaZLBoI4PLEoGSCb0XxbPTNvuRc/PiAC9KcCMu1gGCf2m', 'User'),
  ('diana_prince', 'diana@email.com', '$2b$12$PUyHf6DvHbAMCpJ5QMFpHTDc1YycQUOvwSd/QjBD0LdDNv2hHDg3n', 'User'),
  ('eve_online', 'eve@email.com', '$2b$12$QVzIg7EwIcBNCqK6RNGqIUEd2ZzdRVPvxTe/RkCE1MeDOw3iIEh4o', 'User');

-- =====================
-- 2. INSERT SESSIONS
-- =====================

INSERT INTO sessions (session_name, created_by, max_size) 
VALUES 
  ('General Discussion', 1, 100),      -- Created by admin_sarah
  ('Tech Talk', 1, 50),                -- Created by admin_sarah
  ('Random Chat', 2, 30),              -- Created by mod_james
  ('Gaming Lounge', 2, 75),            -- Created by mod_james
  ('Help & Support', 1, 40);           -- Created by admin_sarah

-- =====================
-- 3. INSERT MESSAGES
-- =====================

-- Messages in General Discussion (session_id: 1)
INSERT INTO messages (session_id, user_id, content) 
VALUES 
  (1, 3, 'Hey everyone! First time here.'),
  (1, 4, 'Welcome alice_wonder! Good to have you here.'),
  (1, 5, 'Hi all! What are we discussing today?'),
  (1, 3, 'Thanks bob_builder! Excited to be part of this community.'),
  (1, 6, 'Anyone know when the next event is?'),
  (1, 4, 'I think it''s scheduled for next week.');

-- Messages in Tech Talk (session_id: 2)
INSERT INTO messages (session_id, user_id, content) 
VALUES 
  (2, 3, 'Has anyone tried Rust for backend development?'),
  (2, 5, 'Yes! I''ve been using it for a few months now. Love it!'),
  (2, 4, 'I''m more of a Go person, but Rust looks interesting.'),
  (2, 3, 'What do you like most about Rust, charlie?'),
  (2, 5, 'The type safety and performance. Plus no garbage collector!'),
  (2, 7, 'Thinking about learning it. Any good resources?');

-- Messages in Random Chat (session_id: 3)
INSERT INTO messages (session_id, user_id, content) 
VALUES 
  (3, 4, 'Good morning everyone!'),
  (3, 6, 'Morning bob! How''s your day going?'),
  (3, 4, 'Pretty good! Just finished a project.'),
  (3, 7, 'Congrats! What was the project about?');

-- Messages in Gaming Lounge (session_id: 4)
INSERT INTO messages (session_id, user_id, content) 
VALUES 
  (4, 5, 'Anyone up for a game tonight?'),
  (4, 6, 'Count me in! What are we playing?'),
  (4, 7, 'I''m free after 8pm'),
  (4, 5, 'Let''s do some co-op adventure game');

-- Messages in Help & Support (session_id: 5)
INSERT INTO messages (session_id, user_id, content) 
VALUES 
  (5, 3, 'How do I change my username?'),
  (5, 1, 'Hi alice! You can change it in your profile settings.'),
  (5, 3, 'Thanks admin_sarah! Found it.'),
  (5, 4, 'Can we create our own sessions?'),
  (5, 2, 'Currently only moderators can create sessions, bob_builder.');

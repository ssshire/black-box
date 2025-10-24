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

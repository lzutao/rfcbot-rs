CREATE TABLE release (
  id SERIAL PRIMARY KEY,
  date DATE UNIQUE NOT NULL,
  released BOOLEAN NOT NULL
)

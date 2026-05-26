
-- Add migration script here
  CREATE TABLE logs (
      original_msg text not null,
      version int,
      prival int not null,
      date text not null,
      hostname text not null,
      appname text not null,
      procid text not null,
      msgid text not null,
      structureddata text not null,
      msg text not null,
      timestamp text not null
  );


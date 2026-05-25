
-- Add migration script here
CREATE TABLE logs (prival int, version int, date text, hostname text, appname text, procid int, msgid text, structureddata text, msg text, original_msg text, timestamp float)

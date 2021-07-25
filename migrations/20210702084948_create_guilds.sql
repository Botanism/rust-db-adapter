-- guild support
create table guilds(
    id bigint primary key,
    welcome_message varchar(2048),
    goodbye_message varchar(2048),
    advertise bool not null,
    admin_chan bigint,
    poll_chans bigint[],
    priv_admin bigint[] not null,
    priv_manager bigint[] not null,
    priv_event bigint[] not null
)

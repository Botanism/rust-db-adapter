-- slaps support
create table slaps(
    sentence bigint primary key,
    guild bigint not null,
    offender bigint not null,
    enforcer bigint,
    reason varchar(2048)
)

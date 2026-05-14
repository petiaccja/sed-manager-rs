//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::{
    spec::{self, column_types::TableKind, objects::TableDesc},
    virtual_device::data::object_table::TableTable,
};

use super::MBR_SIZE;

pub fn preconfig_table() -> TableTable {
    let items = [
        TableDesc {
            uid: spec::core::table::MBR_CONTROL,
            name: "MBRControl".into(),
            kind: TableKind::Object,
            ..Default::default()
        },
        TableDesc {
            uid: spec::core::table::MBR,
            name: "MBR".into(),
            kind: TableKind::Byte,
            rows: MBR_SIZE,
            ..Default::default()
        },
    ];

    items.into_iter().collect()
}

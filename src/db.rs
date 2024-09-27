use std::error::Error;
use tokio_postgres::Client;
use crate::model::{Order, Delivery, Payment, Item};
use log::info;


pub async fn add_order(order: &Order, client: &Client) -> Result<(), Box<dyn Error>> {
    info!("Adding order with ID: {:?}", order.order_uid);

    let delivery_id = insert_delivery(&order.delivery, client).await?;
    insert_payment(&order.payment, client).await?;
    insert_order(order, client, delivery_id).await?;

    for item in &order.items {
        insert_item(item, client).await?;
        insert_order_item(order, item, client).await?;
    }

    info!("Successfully added order with ID: {:?}", order.order_uid);
    Ok(())
}

async fn insert_delivery(delivery: &Delivery, client: &Client) -> Result<i64, Box<dyn Error>> {
    info!("Adding delivery");

    let query = r#"
        INSERT INTO delivery (
            name,
            phone,
            zip,
            city,
            address,
            region,
            email
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING delivery_id
    "#;

    let row = client.query_one(query, &[
        &delivery.name, 
        &delivery.phone, 
        &delivery.zip, 
        &delivery.city, 
        &delivery.address, 
        &delivery.region, 
        &delivery.email
    ]).await?;

    let delivery_id: i64 = row.get(0);

    info!("Successfully added delivery with ID: {:?}", delivery_id);
    Ok(delivery_id)
}

async fn insert_payment(payment: &Payment, client: &Client) -> Result<(), Box<dyn Error>> {
    info!("Adding payment with ID: {:?}", payment.transaction);

    let query = r#"
        INSERT INTO payment (
            transaction,
            request_id,
            currency,
            provider,
            amount,
            payment_dt,
            bank,
            delivery_cost,
            goods_total,
            custom_fee
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    "#;

    client.execute(query, &[
        &payment.transaction,
        &payment.request_id,
        &payment.currency,
        &payment.provider,
        &payment.amount,
        &payment.payment_dt,
        &payment.bank,
        &payment.delivery_cost,
        &payment.goods_total,
        &payment.custom_fee,
    ]).await?;

    info!("Successfully added payment with ID: {:?}", payment.transaction);
    Ok(())
}

async fn insert_order(order: &Order, client: &Client, delivery_id: i64) -> Result<(), Box<dyn Error>> {
    info!("Adding order info with ID: {:?}", order.order_uid);

    let query = r#"
        INSERT INTO order_info (
            order_uid,
            track_number,
            entry,
            delivery_id,
            payment_transaction,
            locale,
            internal_signature,
            customer_id,
            delivery_service,
            shardkey,
            sm_id,
            date_created,
            oof_shard
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
    "#;

    client.execute(query, &[
        &order.order_uid,
        &order.track_number,
        &order.entry,
        &delivery_id,
        &order.payment.transaction,
        &order.locale,
        &order.internal_signature,
        &order.customer_id,
        &order.delivery_service,
        &order.shardkey,
        &order.sm_id,
        &order.date_created,
        &order.oof_shard,
    ]).await?;

    info!("Successfully added order info with ID: {:?}", order.order_uid);
    Ok(())
}

async fn insert_item(item: &Item, client: &Client) -> Result<(), Box<dyn Error>> {
    info!("Adding item with ID: {:?}", item.chrt_id);

    let query = r#"
        INSERT INTO item (
            chrt_id,
            track_number,
            price,
            rid,
            name,
            sale,
            size,
            total_price,
            nm_id,
            brand,
            status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    "#;

    client.execute(query, &[
        &item.chrt_id,
        &item.track_number,
        &item.price,
        &item.rid,
        &item.name,
        &item.sale,
        &item.size,
        &item.total_price,
        &item.nm_id,
        &item.brand,
        &item.status,
    ]).await?;

    info!("Succesfully added item with ID: {:?}", item.chrt_id);
    Ok(())
}

async fn insert_order_item(order: &Order, item: &Item, client: &Client) -> Result<(), Box<dyn Error>> {
    info!("Adding order item with order ID: {:?}, item ID: {:?}", order.order_uid, item.chrt_id);

    let query = r#"
        INSERT INTO order_item (
        order_uid,
        item_chrt_id
        ) VALUES ($1, $2)
    "#;

    client.execute(query, &[&order.order_uid, &item.chrt_id]).await?;

    info!("Successfully added order item with order ID: {:?}, item ID: {:?}", order.order_uid, item.chrt_id);
    Ok(())
}

pub async fn get_order_by_uid(order_uid: &String, client: &Client) -> Result<Order, Box<dyn Error>> {
    info!("Getting order with ID: {:?}", order_uid);
    let query = r#"
            SELECT 
                oi.order_uid, oi.track_number, oi.entry, oi.locale, oi.internal_signature, 
                oi.customer_id, oi.delivery_service, oi.shardkey, oi.sm_id, oi.date_created, 
                oi.oof_shard, d.delivery_id, d.name, d.phone, d.zip, d.city, d.address, 
                d.region, d.email, p.transaction, p.request_id, p.currency, p.provider, 
                p.amount, p.payment_dt, p.bank, p.delivery_cost, p.goods_total, p.custom_fee
            FROM 
                order_info oi
            JOIN 
                delivery d ON oi.delivery_id = d.delivery_id
            JOIN 
                payment p ON oi.payment_transaction = p.transaction
            WHERE 
                oi.order_uid = $1
            "#;
    let row = client.query_one(query,&[&order_uid]).await?;


    let mut order = map_order_from_row(&row);
    order.items = get_items_for_order(&order.order_uid, client).await?;

    info!("Successfully got order with ID: {:?}", order_uid);
    Ok(order)
}

async fn get_items_for_order(order_uid: &String, client: &Client) -> Result<Vec<Item>, Box<dyn Error>> {
    info!("Getting items for order with ID: {:?}", order_uid);

    let query = r#"
            SELECT 
                i.chrt_id, i.track_number, i.price, i.rid, i.name, i.sale, 
                i.size, i.total_price, i.nm_id, i.brand, i.status 
            FROM 
                item i 
            JOIN 
                order_item oi ON i.chrt_id = oi.item_chrt_id 
            WHERE 
                oi.order_uid = $1
            "#;
    
    let rows = client
        .query(query,&[&order_uid],).await?;

    let mut items = Vec::new();

    for row in rows {
        let item = map_item_from_row(&row);
        items.push(item);
    }

    info!("Successfully got items for order with ID: {:?}", order_uid);

    Ok(items)
}

fn map_item_from_row(row: &tokio_postgres::Row) -> Item {
    Item {
        chrt_id: row.get("chrt_id"),
        track_number: row.get("track_number"),
        price: row.get("price"),
        rid: row.get("rid"),
        name: row.get("name"),
        sale: row.get("sale"),
        size: row.get("size"),
        total_price: row.get("total_price"),
        nm_id: row.get("nm_id"),
        brand: row.get("brand"),
        status: row.get("status"),
    }
}

fn map_order_from_row(row: &tokio_postgres::Row) -> Order {
    let delivery =     Delivery {
        name: row.get("name"),
        phone: row.get("phone"),
        zip: row.get("zip"),
        city: row.get("city"),
        address: row.get("address"),
        region: row.get("region"),
        email: row.get("email"),
    };

    let payment =      Payment {
        transaction: row.get("transaction"),
        request_id: row.get("request_id"),
        currency: row.get("currency"),
        provider: row.get("provider"),
        amount: row.get("amount"),
        payment_dt: row.get("payment_dt"),
        bank: row.get("bank"),
        delivery_cost: row.get("delivery_cost"),
        goods_total: row.get("goods_total"),
        custom_fee: row.get("custom_fee"),
    };

    Order {
        order_uid: row.get("order_uid"),
        track_number: row.get("track_number"),
        entry: row.get("entry"),
        locale: row.get("locale"),
        internal_signature: row.get("internal_signature"),
        customer_id: row.get("customer_id"),
        delivery_service: row.get("delivery_service"),
        shardkey: row.get("shardkey"),
        sm_id: row.get("sm_id"),
        date_created: row.get("date_created"),
        oof_shard: row.get("oof_shard"),
        delivery,
        payment,
        items: Vec::new(),
    }
}

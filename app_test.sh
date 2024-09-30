#!/bin/bash

get_order_json() {
    local index=$1

    local json='{
      "order_uid": "b563feb7b2b84b6test",
      "track_number": "WBILMTESTTRACK",
      "entry": "WBIL",
      "delivery": {
        "name": "Test Testov",
        "phone": "+9720000000",
        "zip": "2639809",
        "city": "Kiryat Mozkin",
        "address": "Ploshad Mira 15",
        "region": "Kraiot",
        "email": "test@gmail.com"
      },
      "payment": {
        "transaction": "b563feb7b2b84b6test",
        "request_id": "",
        "currency": "USD",
        "provider": "wbpay",
        "amount": 1817,
        "payment_dt": 1637907727,
        "bank": "alpha",
        "delivery_cost": 1500,
        "goods_total": 317,
        "custom_fee": 0
      },
      "items": [
        {
          "chrt_id": 9934930,
          "track_number": "WBILMTESTTRACK",
          "price": 453,
          "rid": "ab4219087a764ae0btest",
          "name": "Mascaras",
          "sale": 30,
          "size": "0",
          "total_price": 317,
          "nm_id": 2389212,
          "brand": "Vivienne Sabo",
          "status": 202
        }
      ],
      "locale": "en",
      "internal_signature": "",
      "customer_id": "test",
      "delivery_service": "meest",
      "shardkey": "9",
      "sm_id": 99,
      "date_created": "2021-11-26T06:22:19Z",
      "oof_shard": "1"
    }'

    echo "$json" | jq --arg index "$index" '
      .order_uid = $index |
      .payment.transaction = $index |
      .items[0].chrt_id = ($index | tonumber)
    '
}

send_post_request() {
    local json=$1
    local url="http://127.0.0.1:8000/add_order"

    response=$(
        curl -X POST "$url" \
            -H "Content-Type: application/json" \
            -o /dev/null \
            -d "$json" \
            -s \
            -w "%{http_code}"
    )

    echo $response
}

send_get_request() {
    local id=$1
    local url="http://127.0.0.1:8000/get_order/$id"

    response=$(
        curl -X GET "$url" \
            -H "Content-Type: application/json" \
            -w "%{http_code}" \
            -s
    )

    echo $response
}

echo "Database reset"
yes | sqlx database reset

echo "Build app"
cargo build --release

echo "Run app"
cargo run --release &
PID=$!

sleep 5

for i in {1..100}; do 
    echo "Test $i"

    json=$(get_order_json "$i")
    response_post=$(send_post_request "$json")

    if [[ "$response_post" -ne 200 ]] ; then
        echo "Error POST request, http-code: $response_post"
        kill $PID
        exit 1
    fi


    response_get=$(send_get_request "$i")
    http_code="${response_get: -3}"
    json_response="${response_get:0:${#response_get}-3}"

    if [[ "$http_code" -ne 200 ]] ; then
        echo "Error GET get request, http-code: $http_code"
        kill $PID
        exit 1
    fi

    if ! diff <(echo "$json" | jq -S .) <(echo "$json_response" | jq -S .); then
        echo "JSON do not match for $i order"
        kill $PID
        exit 1
    fi

done 

kill $PID

echo "Success"

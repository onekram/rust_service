echo "Database reset"
yes | sqlx database reset

echo "Build app"
cargo build --release

echo "Run app"
cargo run --release &
PID=$!
sleep 5

response_post=$(
    curl -X POST "http://127.0.0.1:8000/add_order" \
        -H "Content-Type: application/json" \
        -o /dev/null \
        -d @test/model.json \
        -s \
        -w "%{http_code}"
)

if [[ "$response_post" -ne 200 ]] ; then
    echo "Error POST request, http-code: $response_post"
    kill $PID
    exit 1
fi

printf "GET http://127.0.0.1:8000/get_order/b563feb7b2b84b6test
Content-Type: application/json
@test/model.json" > test/target.list

vegeta attack -duration=10s -rate=1000 -targets=test/target.list | vegeta report

rm test/target.list

kill $PID

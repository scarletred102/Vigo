// Copyright (c) 2025 Vigo Browser. All rights reserved.
// Proprietary and confidential. Unauthorized copying prohibited.

// Vigo Sync Server — zero-knowledge self-hosted sync backend.
//
// This server stores only opaque encrypted blobs. It has no ability
// to read, decrypt, or interpret any user data. All cryptographic
// operations happen client-side in the browser.

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

// ─── Configuration ──────────────────────────────────────────────────────────

type Config struct {
	Port     string
	DBPath   string
	LogLevel string
	TLSCert  string
	TLSKey   string
}

func loadConfig() Config {
	return Config{
		Port:     envOrDefault("VIGO_SYNC_PORT", "8443"),
		DBPath:   envOrDefault("VIGO_SYNC_DB_PATH", "./data/vigo_sync.db"),
		LogLevel: envOrDefault("VIGO_SYNC_LOG_LEVEL", "info"),
		TLSCert:  os.Getenv("VIGO_SYNC_TLS_CERT"),
		TLSKey:   os.Getenv("VIGO_SYNC_TLS_KEY"),
	}
}

func envOrDefault(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

// ─── Database ───────────────────────────────────────────────────────────────

var db *sql.DB

func initDB(path string) {
	var err error
	db, err = sql.Open("sqlite3", path+"?_journal_mode=WAL&_busy_timeout=5000")
	if err != nil {
		log.Fatalf("failed to open database: %v", err)
	}

	schema := `
	CREATE TABLE IF NOT EXISTS devices (
		device_id     TEXT PRIMARY KEY,
		device_name   TEXT NOT NULL,
		kx_public_key TEXT NOT NULL,
		verify_key    TEXT NOT NULL,
		auth_token    TEXT NOT NULL,
		registered_at INTEGER NOT NULL,
		last_seen_at  INTEGER NOT NULL,
		is_revoked    INTEGER NOT NULL DEFAULT 0
	);

	CREATE TABLE IF NOT EXISTS records (
		record_id    TEXT NOT NULL,
		collection   TEXT NOT NULL,
		ciphertext   TEXT NOT NULL,
		version      INTEGER NOT NULL DEFAULT 1,
		modified_at  INTEGER NOT NULL,
		content_hash TEXT,
		device_id    TEXT,
		PRIMARY KEY (record_id, collection)
	);

	CREATE INDEX IF NOT EXISTS idx_records_collection_modified
	ON records(collection, modified_at);

	CREATE TABLE IF NOT EXISTS wrapped_keys (
		target_device_id TEXT PRIMARY KEY,
		wrapped_key      TEXT NOT NULL,
		created_at       INTEGER NOT NULL
	);
	`

	if _, err := db.Exec(schema); err != nil {
		log.Fatalf("failed to create schema: %v", err)
	}

	log.Println("Database initialised at", path)
}

// ─── Models ─────────────────────────────────────────────────────────────────

type DeviceRegistrationReq struct {
	DeviceName  string `json:"device_name"`
	KxPublicKey string `json:"kx_public_key"`
	VerifyKey   string `json:"verify_key"`
	Signature   string `json:"signature"`
}

type DeviceRegistrationResp struct {
	DeviceID       string `json:"device_id"`
	AuthToken      string `json:"auth_token"`
	WrappedRootKey string `json:"wrapped_root_key,omitempty"`
}

type RecordReq struct {
	RecordID    string `json:"record_id"`
	Collection  string `json:"collection"`
	Ciphertext  string `json:"ciphertext"`
	Version     int    `json:"version"`
	ModifiedAt  string `json:"modified_at"`
	ContentHash string `json:"content_hash,omitempty"`
}

type RecordListResp struct {
	Records []RecordReq `json:"records"`
}

type WrappedKeyReq struct {
	TargetDeviceID string `json:"target_device_id"`
	WrappedKey     string `json:"wrapped_key"`
}

// ─── Handlers ───────────────────────────────────────────────────────────────

func healthHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status":  "ok",
		"version": "0.1.0",
	})
}

func registerDeviceHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req DeviceRegistrationReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	// Generate device ID and auth token.
	deviceID := fmt.Sprintf("dev_%d", time.Now().UnixNano())
	authToken := fmt.Sprintf("tok_%d", time.Now().UnixNano())
	now := time.Now().UnixMilli()

	_, err := db.Exec(
		`INSERT INTO devices (device_id, device_name, kx_public_key, verify_key,
		                       auth_token, registered_at, last_seen_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?)`,
		deviceID, req.DeviceName, req.KxPublicKey, req.VerifyKey,
		authToken, now, now,
	)
	if err != nil {
		log.Printf("device registration error: %v", err)
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	resp := DeviceRegistrationResp{
		DeviceID:  deviceID,
		AuthToken: authToken,
	}

	// Check if there's a wrapped key waiting for this device.
	var wrappedKey string
	err = db.QueryRow(
		"SELECT wrapped_key FROM wrapped_keys WHERE target_device_id = ?",
		deviceID).Scan(&wrappedKey)
	if err == nil {
		resp.WrappedRootKey = wrappedKey
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func deregisterDeviceHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	deviceID := strings.TrimPrefix(r.URL.Path, "/api/v1/devices/")
	if deviceID == "" {
		http.Error(w, "missing device_id", http.StatusBadRequest)
		return
	}

	_, err := db.Exec(
		"UPDATE devices SET is_revoked = 1 WHERE device_id = ?", deviceID)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func pushRecordHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPut {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Extract collection from path: /api/v1/collections/{type}/records
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 5 {
		http.Error(w, "bad path", http.StatusBadRequest)
		return
	}
	collection := parts[4]

	// Check if this is a batch request.
	if len(parts) > 6 && parts[6] == "batch" {
		pushBatchHandler(w, r, collection)
		return
	}

	var req RecordReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	modifiedAt, _ := strconv.ParseInt(req.ModifiedAt, 10, 64)
	if modifiedAt == 0 {
		modifiedAt = time.Now().UnixMilli()
	}

	_, err := db.Exec(
		`INSERT OR REPLACE INTO records
		 (record_id, collection, ciphertext, version, modified_at, content_hash)
		 VALUES (?, ?, ?, ?, ?, ?)`,
		req.RecordID, collection, req.Ciphertext,
		req.Version, modifiedAt, req.ContentHash,
	)
	if err != nil {
		log.Printf("push record error: %v", err)
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func pushBatchHandler(w http.ResponseWriter, r *http.Request, collection string) {
	var batch struct {
		Records []RecordReq `json:"records"`
	}
	if err := json.NewDecoder(r.Body).Decode(&batch); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	tx, err := db.Begin()
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	stmt, err := tx.Prepare(
		`INSERT OR REPLACE INTO records
		 (record_id, collection, ciphertext, version, modified_at, content_hash)
		 VALUES (?, ?, ?, ?, ?, ?)`)
	if err != nil {
		tx.Rollback()
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}
	defer stmt.Close()

	for _, rec := range batch.Records {
		modifiedAt, _ := strconv.ParseInt(rec.ModifiedAt, 10, 64)
		if modifiedAt == 0 {
			modifiedAt = time.Now().UnixMilli()
		}
		if _, err := stmt.Exec(
			rec.RecordID, collection, rec.Ciphertext,
			rec.Version, modifiedAt, rec.ContentHash,
		); err != nil {
			tx.Rollback()
			http.Error(w, "internal error", http.StatusInternalServerError)
			return
		}
	}

	if err := tx.Commit(); err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func pullRecordsHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 5 {
		http.Error(w, "bad path", http.StatusBadRequest)
		return
	}
	collection := parts[4]

	since := int64(0)
	if s := r.URL.Query().Get("since"); s != "" {
		since, _ = strconv.ParseInt(s, 10, 64)
	}

	rows, err := db.Query(
		`SELECT record_id, collection, ciphertext, version, modified_at,
		        COALESCE(content_hash, '')
		 FROM records
		 WHERE collection = ? AND modified_at > ?
		 ORDER BY modified_at ASC`,
		collection, since,
	)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var records []RecordReq
	for rows.Next() {
		var rec RecordReq
		var modifiedAt int64
		if err := rows.Scan(
			&rec.RecordID, &rec.Collection, &rec.Ciphertext,
			&rec.Version, &modifiedAt, &rec.ContentHash,
		); err != nil {
			continue
		}
		rec.ModifiedAt = strconv.FormatInt(modifiedAt, 10)
		records = append(records, rec)
	}

	if records == nil {
		records = []RecordReq{}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(RecordListResp{Records: records})
}

func deleteRecordHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 7 {
		http.Error(w, "bad path", http.StatusBadRequest)
		return
	}
	collection := parts[4]
	recordID := parts[6]

	_, err := db.Exec(
		"DELETE FROM records WHERE record_id = ? AND collection = ?",
		recordID, collection,
	)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func pushWrappedKeyHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req WrappedKeyReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "bad request", http.StatusBadRequest)
		return
	}

	_, err := db.Exec(
		`INSERT OR REPLACE INTO wrapped_keys
		 (target_device_id, wrapped_key, created_at)
		 VALUES (?, ?, ?)`,
		req.TargetDeviceID, req.WrappedKey, time.Now().UnixMilli(),
	)
	if err != nil {
		http.Error(w, "internal error", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func fetchWrappedKeyHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	deviceID := strings.TrimPrefix(r.URL.Path, "/api/v1/keys/wrap/")
	if deviceID == "" {
		http.Error(w, "missing device_id", http.StatusBadRequest)
		return
	}

	var wrappedKey string
	err := db.QueryRow(
		"SELECT wrapped_key FROM wrapped_keys WHERE target_device_id = ?",
		deviceID).Scan(&wrappedKey)
	if err != nil {
		http.Error(w, "not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"wrapped_key": wrappedKey,
	})
}

// ─── Router ─────────────────────────────────────────────────────────────────

func setupRouter() http.Handler {
	mux := http.NewServeMux()

	mux.HandleFunc("/api/v1/health", healthHandler)
	mux.HandleFunc("/api/v1/devices", registerDeviceHandler)
	mux.HandleFunc("/api/v1/devices/", deregisterDeviceHandler)
	mux.HandleFunc("/api/v1/keys/wrap", pushWrappedKeyHandler)
	mux.HandleFunc("/api/v1/keys/wrap/", fetchWrappedKeyHandler)

	// Collection routes use a single handler that dispatches on method.
	mux.HandleFunc("/api/v1/collections/", func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodPut:
			pushRecordHandler(w, r)
		case http.MethodGet:
			pullRecordsHandler(w, r)
		case http.MethodDelete:
			deleteRecordHandler(w, r)
		default:
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		}
	})

	return mux
}

// ─── Main ───────────────────────────────────────────────────────────────────

func main() {
	config := loadConfig()

	// Ensure data directory exists.
	os.MkdirAll("./data", 0755)

	initDB(config.DBPath)
	defer db.Close()

	handler := setupRouter()

	addr := ":" + config.Port
	log.Printf("Vigo Sync Server starting on %s", addr)

	server := &http.Server{
		Addr:         addr,
		Handler:      handler,
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	if config.TLSCert != "" && config.TLSKey != "" {
		log.Printf("TLS enabled: cert=%s key=%s", config.TLSCert, config.TLSKey)
		log.Fatal(server.ListenAndServeTLS(config.TLSCert, config.TLSKey))
	} else {
		log.Println("WARNING: TLS disabled — use a reverse proxy in production")
		log.Fatal(server.ListenAndServe())
	}
}

-- Create permission_audit_log table for security auditing
-- Immutable log of all permission-related changes

CREATE TABLE permission_audit_log (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type TEXT NOT NULL,
    actor_id UUID,
    resource_type TEXT,
    resource_id UUID,
    subject_type TEXT,
    subject_id UUID,
    permission_level TEXT,
    details JSONB NOT NULL DEFAULT '{}',
    PRIMARY KEY (timestamp, id)
) PARTITION BY RANGE (timestamp);

-- Create initial partition for current month and next 12 months
DO $$
DECLARE
    start_date DATE := DATE_TRUNC('month', CURRENT_DATE);
    end_date DATE;
    partition_name TEXT;
BEGIN
    FOR i IN 0..12 LOOP
        end_date := start_date + INTERVAL '1 month';
        partition_name := 'permission_audit_log_' || TO_CHAR(start_date, 'YYYY_MM');

        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF permission_audit_log
             FOR VALUES FROM (%L) TO (%L)',
            partition_name, start_date, end_date
        );

        start_date := end_date;
    END LOOP;
END $$;

-- Index for finding audit events by actor
CREATE INDEX idx_audit_actor ON permission_audit_log(actor_id, timestamp DESC);

-- Index for finding audit events by resource
CREATE INDEX idx_audit_resource ON permission_audit_log(resource_type, resource_id, timestamp DESC);

-- Index for event type queries
CREATE INDEX idx_audit_event_type ON permission_audit_log(event_type, timestamp DESC);

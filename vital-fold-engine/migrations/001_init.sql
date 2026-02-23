-- VitalFold Engine - Initial Database Schema
-- PostgreSQL/Aurora DSQL Migration
-- Creates public.users table for authentication

-- Create the public schema (it should exist by default)
CREATE SCHEMA IF NOT EXISTS public;

-- Users table for authentication
CREATE TABLE IF NOT EXISTS public.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on email for fast lookups during login
CREATE INDEX IF NOT EXISTS idx_users_email ON public.users(email);

-- Note: The vital_fold schema and its tables are created separately
-- via the health_clinic_schema.sql file during initial setup.
-- This migration focuses only on the authentication table needed by the API.

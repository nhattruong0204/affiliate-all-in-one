#!/usr/bin/env bash
# First-boot hardening for a fresh Ubuntu LTS VPS (AAO-092, docs/INFRASTRUCTURE.md §2). Run once as
# root; safe to re-run.
#
#   DEPLOY_SSH_KEY_FILE=/root/deploy.pub infra/bootstrap/vps-bootstrap.sh
#
# SSH stays on port 22 (Ubuntu 24.04 starts sshd through ssh.socket, which ignores `Port` in
# sshd_config, so a custom port needs a different procedure than this script).
#
# What it does: non-root deploy user (key only), SSH hardening, ufw deny-all except 22/80/443 TCP,
# fail2ban for sshd, unattended security upgrades, Docker Engine + Compose plugin from the Ubuntu
# archive.
#
# Safety: it refuses to turn off password login unless a public key for the deploy user is
# installed, and validates sshd's configuration before reloading, so a typo cannot lock you out.
# Keep your current SSH session open and test a second login before closing it.
#
# NOTE: Docker publishes container ports by editing iptables directly, which bypasses ufw. The
# firewall is therefore not what keeps Postgres, Prometheus and the metrics port private: the
# compose file simply never publishes them (checked by scripts/check-infra.sh).
set -euo pipefail

deploy_user="${DEPLOY_USER:-deploy}"
key_file="${DEPLOY_SSH_KEY_FILE:?set DEPLOY_SSH_KEY_FILE to a file holding the deploy user SSH public key}"
install_dir="${AAO_INSTALL_DIR:-/opt/aao}"

[[ $EUID -eq 0 ]] || { echo "run as root" >&2; exit 1; }
[[ "$deploy_user" =~ ^[a-z][a-z0-9_-]{1,31}$ ]] || { echo "invalid DEPLOY_USER" >&2; exit 1; }
grep -Eq '^(ssh-(ed25519|rsa)|ecdsa-sha2-[a-z0-9]+|sk-ssh-ed25519@openssh\.com) ' "$key_file" \
  || { echo "$key_file does not look like an SSH public key" >&2; exit 1; }

export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq ufw fail2ban unattended-upgrades docker.io docker-compose-v2 ca-certificates curl jq

# --- deploy user -----------------------------------------------------------------------------
if ! id "$deploy_user" >/dev/null 2>&1; then
  adduser --disabled-password --gecos "" "$deploy_user"
fi
# Membership of `docker` is root-equivalent on this host: that is the deploy user's purpose.
usermod -aG docker "$deploy_user"
install -d -m 700 -o "$deploy_user" -g "$deploy_user" "/home/$deploy_user/.ssh"
touch "/home/$deploy_user/.ssh/authorized_keys"
while IFS= read -r key; do
  [[ -z "$key" || "$key" == \#* ]] && continue
  grep -qxF "$key" "/home/$deploy_user/.ssh/authorized_keys" || echo "$key" >>"/home/$deploy_user/.ssh/authorized_keys"
done <"$key_file"
chown "$deploy_user:$deploy_user" "/home/$deploy_user/.ssh/authorized_keys"
chmod 600 "/home/$deploy_user/.ssh/authorized_keys"
install -d -m 755 -o "$deploy_user" -g "$deploy_user" "$install_dir"

# --- SSH: keys only, no root ------------------------------------------------------------------
cat >/etc/ssh/sshd_config.d/10-aao-hardening.conf <<CONF
PermitRootLogin no
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
X11Forwarding no
MaxAuthTries 3
AllowUsers ${deploy_user}
CONF
sshd -t
systemctl reload ssh || systemctl reload sshd

# --- firewall: 22/80/443 TCP only --------------------------------------------------------------
ufw --force reset >/dev/null
ufw default deny incoming
ufw default allow outgoing
ufw limit 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw --force enable

# --- fail2ban for sshd ---------------------------------------------------------------------------
cat >/etc/fail2ban/jail.d/aao.local <<CONF
[sshd]
enabled = true
port = 22
maxretry = 5
findtime = 10m
bantime = 1h
CONF
systemctl enable --now fail2ban
systemctl restart fail2ban

# --- unattended security upgrades -----------------------------------------------------------------
cat >/etc/apt/apt.conf.d/20auto-upgrades <<CONF
APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Unattended-Upgrade "1";
CONF
systemctl enable --now unattended-upgrades

# --- docker: rotate container logs ------------------------------------------------------------------
if [[ ! -f /etc/docker/daemon.json ]]; then
  cat >/etc/docker/daemon.json <<'CONF'
{ "log-driver": "json-file", "log-opts": { "max-size": "10m", "max-file": "5" } }
CONF
fi
systemctl enable --now docker
systemctl restart docker

echo "bootstrap done. Test a new SSH login as ${deploy_user} before closing this session."
echo "Next: clone the repository to ${install_dir}, then run infra/up.sh as ${deploy_user}."

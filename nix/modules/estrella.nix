{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.estrella;
in
{
  options.services.estrella = {
    enable = mkEnableOption "Estrella thermal printer HTTP daemon";

    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port to listen on";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = ''
        Address to bind to.
        - "0.0.0.0" for all interfaces (public access)
        - "127.0.0.1" for localhost only (more secure)
      '';
    };

    deviceMac = mkOption {
      type = types.str;
      description = "Bluetooth MAC address of the thermal printer";
    };

    rfcommChannel = mkOption {
      type = types.int;
      default = 0;
      description = "RFCOMM channel number (creates /dev/rfcommN)";
    };

    package = mkOption {
      type = types.package;
      default = pkgs.estrella or (throw ''
        estrella package not found in pkgs.
        You need to add the overlay to your nixpkgs.overlays:
          nixpkgs.overlays = [ inputs.estrella.overlays.default ];
      '');
      description = "Estrella package to use";
    };
  };

  config = mkIf cfg.enable {
    # Make estrella available system-wide
    environment.systemPackages = [ cfg.package ];

    # RFCOMM setup service (runs as root to bind rfcomm)
    systemd.services.estrella-rfcomm = {
      description = "Estrella RFCOMM Setup";
      documentation = [ "https://github.com/eljojo/estrella" ];
      after = [ "bluetooth.target" ];
      before = [ "estrella.service" ];
      requiredBy = [ "estrella.service" ];

      path = [ pkgs.bluez ];

      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        ExecStart = "${cfg.package}/bin/estrella setup-rfcomm ${cfg.deviceMac} --channel ${toString cfg.rfcommChannel}";
      };
    };

    # Main HTTP daemon service (runs unprivileged)
    systemd.services.estrella = {
      description = "Estrella Thermal Printer HTTP Daemon";
      documentation = [ "https://github.com/eljojo/estrella" ];
      after = [ "network.target" "estrella-rfcomm.service" ];
      requires = [ "estrella-rfcomm.service" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/estrella serve --listen ${cfg.listenAddress}:${toString cfg.port} --device /dev/rfcomm${toString cfg.rfcommChannel}";
        Restart = "always";
        RestartSec = "10s";

        # Security hardening
        DynamicUser = true;
        SupplementaryGroups = [ "dialout" ];  # For /dev/rfcomm access

        # Sandboxing (allow device access)
        PrivateTmp = true;
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictRealtime = true;
        RestrictNamespaces = true;
        LockPersonality = true;

        # Resource limits
        MemoryMax = "256M";
        TasksMax = 128;
      };
    };

    # Open firewall port if listening on non-localhost
    networking.firewall.allowedTCPPorts = mkIf (cfg.listenAddress != "127.0.0.1") [ cfg.port ];
  };
}

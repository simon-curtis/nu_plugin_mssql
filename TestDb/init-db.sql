-- Create the TestDB database if it doesn't already exist
IF NOT EXISTS (SELECT * FROM sys.databases WHERE name = 'TestDB')
BEGIN
    CREATE DATABASE TestDB;
END
GO

-- Use the TestDB database
USE TestDB;
GO

-- Create the Users table if it doesn't already exist
IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'Users')
BEGIN
    CREATE TABLE Users (
        UserID INT PRIMARY KEY IDENTITY(1,1),
        FirstName NVARCHAR(50),
        LastName NVARCHAR(50),
        Email NVARCHAR(100),
        DateOfBirth DATE
    );
END
GO

-- Create the Pokemon table
IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'Pokemon')
BEGIN
    CREATE TABLE Pokemon (
        PokemonID INT PRIMARY KEY IDENTITY(1,1),
        Name NVARCHAR(50),
        Type NVARCHAR(50),
        Level INT,
        TrainerID INT,
        FOREIGN KEY (TrainerID) REFERENCES Users(UserID)
    );
END
GO

-- Create the Items table
IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'Items')
BEGIN
    CREATE TABLE Items (
        ItemID INT PRIMARY KEY IDENTITY(1,1),
        ItemName NVARCHAR(50),
        ItemType NVARCHAR(50),
        Quantity INT,
        OwnerID INT,
        FOREIGN KEY (OwnerID) REFERENCES Users(UserID)
    );
END
GO

-- Insert Pokémon-themed fake data into the Users table
INSERT INTO Users (FirstName, LastName, Email, DateOfBirth)
VALUES 
('Ash', 'Ketchum', 'ash.ketchum@pokemon.com', '1987-05-22'),
('Misty', 'Waterflower', 'misty.waterflower@pokemon.com', '1989-08-03'),
('Brock', 'Harrison', 'brock.harrison@pokemon.com', '1985-07-15'),
('Gary', 'Oak', 'gary.oak@pokemon.com', '1987-06-01'),
('Serena', 'Gabena', 'serena.gabena@pokemon.com', '1992-07-10'),
('Dawn', 'Berlitz', 'dawn.berlitz@pokemon.com', '1990-04-20'),
('May', 'Maple', 'may.maple@pokemon.com', '1988-09-30'),
('Clemont', 'Lemon', 'clemont.lemon@pokemon.com', '1991-03-12'),
('Bonnie', 'Lemon', 'bonnie.lemon@pokemon.com', '2003-12-21'),
('Tracey', 'Sketchit', 'tracey.sketchit@pokemon.com', '1987-09-15');
GO

-- Insert data into the Pokemon table
INSERT INTO Pokemon (Name, Type, Level, TrainerID)
VALUES 
('Pikachu', 'Electric', 50, 1),
('Bulbasaur', 'Grass/Poison', 30, 1),
('Starmie', 'Water/Psychic', 40, 2),
('Psyduck', 'Water', 25, 2),
('Onix', 'Rock/Ground', 45, 3),
('Geodude', 'Rock/Ground', 20, 3),
('Blastoise', 'Water', 60, 4),
('Arcanine', 'Fire', 50, 4),
('Braixen', 'Fire', 35, 5),
('Pancham', 'Fighting', 20, 5);
GO

-- Insert data into the Items table
INSERT INTO Items (ItemName, ItemType, Quantity, OwnerID)
VALUES 
('Pokeball', 'Capture', 10, 1),
('Potion', 'Healing', 5, 1),
('Super Potion', 'Healing', 3, 2),
('Water Stone', 'Evolution', 1, 2),
('Revive', 'Healing', 2, 3),
('Full Restore', 'Healing', 1, 3),
('Fire Stone', 'Evolution', 1, 4),
('Rare Candy', 'Misc', 5, 4),
('Escape Rope', 'Misc', 2, 5),
('Antidote', 'Healing', 4, 5);
GO